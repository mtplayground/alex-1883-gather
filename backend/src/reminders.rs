use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use tokio::time::{self, MissedTickBehavior};

use crate::email::{templates, EmailDispatcher, EmailError, EmailMessage};

const REMINDER_KIND_24H: &str = "24h";
const REMINDER_LOOKAHEAD_HOURS: i64 = 24;
const REMINDER_TICK_SECONDS: u64 = 15 * 60;

#[derive(Clone)]
pub struct ReminderRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, FromRow)]
pub struct ReminderCandidate {
    pub event_id: String,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub timezone: Option<String>,
    pub recipient_sub: String,
    pub recipient_email: String,
    pub recipient_name: Option<String>,
}

#[derive(Debug)]
pub struct ReminderDeliveryStatus {
    status: &'static str,
    email_dispatch_id: Option<String>,
    message: Option<String>,
}

impl ReminderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn due_candidates(&self) -> Result<Vec<ReminderCandidate>, sqlx::Error> {
        sqlx::query_as::<_, ReminderCandidate>(
            r#"
            WITH reminder_events AS (
                SELECT id, owner_sub, title, starts_at, timezone
                FROM events
                WHERE starts_at > NOW()
                    AND starts_at <= NOW() + ($1::text || ' hours')::interval
            ),
            recipients AS (
                SELECT
                    reminder_events.id AS event_id,
                    reminder_events.title,
                    reminder_events.starts_at,
                    reminder_events.timezone,
                    users.sub AS recipient_sub,
                    users.email AS recipient_email,
                    COALESCE(profiles.display_name, users.name) AS recipient_name
                FROM reminder_events
                JOIN users ON users.sub = reminder_events.owner_sub
                LEFT JOIN profiles ON profiles.user_sub = users.sub

                UNION

                SELECT
                    reminder_events.id AS event_id,
                    reminder_events.title,
                    reminder_events.starts_at,
                    reminder_events.timezone,
                    users.sub AS recipient_sub,
                    users.email AS recipient_email,
                    COALESCE(profiles.display_name, users.name) AS recipient_name
                FROM reminder_events
                JOIN event_invitations
                    ON event_invitations.event_id = reminder_events.id
                    AND event_invitations.status = 'accepted'
                    AND event_invitations.invitee_sub IS NOT NULL
                JOIN users ON users.sub = event_invitations.invitee_sub
                LEFT JOIN profiles ON profiles.user_sub = users.sub
                LEFT JOIN event_rsvps
                    ON event_rsvps.invitation_id = event_invitations.id
                WHERE COALESCE(event_rsvps.response, 'yes') IN ('yes', 'maybe')
            )
            SELECT
                recipients.event_id,
                recipients.title,
                recipients.starts_at,
                recipients.timezone,
                recipients.recipient_sub,
                recipients.recipient_email,
                recipients.recipient_name
            FROM recipients
            WHERE NOT EXISTS (
                SELECT 1
                FROM event_reminder_deliveries
                WHERE event_reminder_deliveries.event_id = recipients.event_id
                    AND event_reminder_deliveries.recipient_key = ('user:' || recipients.recipient_sub)
                    AND event_reminder_deliveries.reminder_kind = $2
            )
            ORDER BY recipients.starts_at ASC, recipients.event_id ASC
            LIMIT 200
            "#,
        )
        .bind(REMINDER_LOOKAHEAD_HOURS.to_string())
        .bind(REMINDER_KIND_24H)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn claim_delivery(&self, candidate: &ReminderCandidate) -> Result<bool, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let recipient_key = recipient_key(candidate);
        let result = sqlx::query(
            r#"
            INSERT INTO event_reminder_deliveries (
                id,
                event_id,
                recipient_key,
                recipient_email,
                reminder_kind,
                status
            )
            VALUES ($1, $2, $3, $4, $5, 'pending')
            ON CONFLICT (event_id, recipient_key, reminder_kind)
            DO NOTHING
            "#,
        )
        .bind(id)
        .bind(&candidate.event_id)
        .bind(recipient_key)
        .bind(&candidate.recipient_email)
        .bind(REMINDER_KIND_24H)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn finish_delivery(
        &self,
        candidate: &ReminderCandidate,
        delivery: &ReminderDeliveryStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE event_reminder_deliveries
            SET
                status = $4,
                email_dispatch_id = $5,
                message = $6,
                updated_at = NOW()
            WHERE event_id = $1
                AND recipient_key = $2
                AND reminder_kind = $3
            "#,
        )
        .bind(&candidate.event_id)
        .bind(recipient_key(candidate))
        .bind(REMINDER_KIND_24H)
        .bind(delivery.status)
        .bind(&delivery.email_dispatch_id)
        .bind(&delivery.message)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }
}

pub fn spawn_scheduler(pool: PgPool, email: EmailDispatcher, self_url: String) {
    tokio::spawn(async move {
        let repository = ReminderRepository::new(pool);

        if let Err(error) = process_due_reminders(&repository, &email, &self_url).await {
            tracing::error!(%error, "event reminder scheduler run failed");
        }

        let mut interval = time::interval(Duration::from_secs(REMINDER_TICK_SECONDS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            if let Err(error) = process_due_reminders(&repository, &email, &self_url).await {
                tracing::error!(%error, "event reminder scheduler run failed");
            }
        }
    });
}

pub async fn process_due_reminders(
    repository: &ReminderRepository,
    email: &EmailDispatcher,
    self_url: &str,
) -> Result<usize, sqlx::Error> {
    let candidates = repository.due_candidates().await?;
    let mut attempted = 0;

    for candidate in candidates {
        if !repository.claim_delivery(&candidate).await? {
            continue;
        }

        let delivery = send_reminder(email, self_url, &candidate).await;
        repository.finish_delivery(&candidate, &delivery).await?;
        attempted += 1;
    }

    if attempted > 0 {
        tracing::info!(attempted, "event reminder scheduler processed reminders");
    }

    Ok(attempted)
}

async fn send_reminder(
    email: &EmailDispatcher,
    self_url: &str,
    candidate: &ReminderCandidate,
) -> ReminderDeliveryStatus {
    let event_url = event_url(self_url, &candidate.event_id);
    let starts_at = format_event_start(candidate.starts_at, candidate.timezone.as_deref());
    let name = candidate
        .recipient_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("there");
    let subject = format!("Almost time: {}", candidate.title);
    let html = templates::event_reminder_html(&candidate.title, name, &starts_at, &event_url);
    let text = templates::event_reminder_text(&candidate.title, name, &starts_at, &event_url);
    let message = EmailMessage::new(candidate.recipient_email.clone(), subject)
        .html(html)
        .text(text);

    match email.send(message).await {
        Ok(Some(dispatch)) => ReminderDeliveryStatus {
            status: "sent",
            email_dispatch_id: Some(dispatch.id),
            message: None,
        },
        Ok(None) => ReminderDeliveryStatus {
            status: "skipped",
            email_dispatch_id: None,
            message: Some("email proxy is not configured".to_string()),
        },
        Err(EmailError::RateLimited) => ReminderDeliveryStatus {
            status: "rate_limited",
            email_dispatch_id: None,
            message: Some("try again shortly".to_string()),
        },
        Err(error) => {
            tracing::error!(
                %error,
                event_id = %candidate.event_id,
                recipient_email = %candidate.recipient_email,
                "event reminder email failed"
            );
            ReminderDeliveryStatus {
                status: "failed",
                email_dispatch_id: None,
                message: Some("reminder email could not be sent".to_string()),
            }
        }
    }
}

fn recipient_key(candidate: &ReminderCandidate) -> String {
    format!("user:{}", candidate.recipient_sub)
}

fn event_url(self_url: &str, event_id: &str) -> String {
    format!("{}/events/{}", self_url.trim_end_matches('/'), event_id)
}

fn format_event_start(starts_at: DateTime<Utc>, timezone: Option<&str>) -> String {
    let mut formatted = starts_at.format("%b %-d, %Y at %-I:%M %p UTC").to_string();

    if let Some(timezone) = timezone.filter(|timezone| !timezone.trim().is_empty()) {
        formatted.push_str(" (");
        formatted.push_str(timezone.trim());
        formatted.push(')');
    }

    formatted
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{event_url, format_event_start};

    #[test]
    fn event_url_trims_base_slash() {
        assert_eq!(
            event_url("https://example.com/", "event-1"),
            "https://example.com/events/event-1"
        );
    }

    #[test]
    fn event_start_mentions_utc_and_timezone_hint() {
        let starts_at = chrono::Utc
            .with_ymd_and_hms(2026, 7, 18, 20, 30, 0)
            .unwrap();
        let formatted = format_event_start(starts_at, Some("America/New_York"));

        assert!(formatted.contains("UTC"));
        assert!(formatted.contains("America/New_York"));
    }
}
