use std::{error::Error, fmt};

use reqwest::{Client, StatusCode};

use crate::config::EmailConfig;

#[derive(Clone)]
pub struct EmailDispatcher {
    client: Client,
    url: Option<String>,
    app_token: Option<String>,
    sender_name: String,
}

#[derive(Clone, Debug)]
pub struct EmailMessage {
    pub to: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EmailDispatch {
    pub id: String,
}

#[derive(Debug)]
pub enum EmailError {
    EmptyRecipients,
    EmptySubject,
    EmptyBody,
    RateLimited,
    Request(reqwest::Error),
    ProxyResponse { status: StatusCode, body: String },
}

#[derive(serde::Serialize)]
struct EmailPayload<'a> {
    to: &'a [String],
    subject: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct EmailResponse {
    id: String,
}

impl EmailDispatcher {
    pub fn from_config(config: &EmailConfig) -> Self {
        Self {
            client: Client::new(),
            url: config.url.clone(),
            app_token: config.app_token.clone(),
            sender_name: config.sender_name.clone(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.url.is_some() && self.app_token.is_some()
    }

    pub fn sender_name(&self) -> &str {
        &self.sender_name
    }

    pub async fn send(&self, message: EmailMessage) -> Result<Option<EmailDispatch>, EmailError> {
        message.validate()?;

        let (Some(url), Some(app_token)) = (&self.url, &self.app_token) else {
            tracing::warn!("email proxy is not configured; skipping transactional email");
            return Ok(None);
        };

        let payload = EmailPayload {
            to: &message.to,
            subject: &message.subject,
            html: message.html.as_deref(),
            text: message.text.as_deref(),
            reply_to: message.reply_to.as_deref(),
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(app_token)
            .json(&payload)
            .send()
            .await
            .map_err(EmailError::Request)?;
        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(EmailError::RateLimited);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(EmailError::ProxyResponse { status, body });
        }

        let body = response
            .json::<EmailResponse>()
            .await
            .map_err(EmailError::Request)?;
        Ok(Some(EmailDispatch { id: body.id }))
    }
}

impl EmailMessage {
    pub fn new(to: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            to: vec![to.into()],
            subject: subject.into(),
            html: None,
            text: None,
            reply_to: None,
        }
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    fn validate(&self) -> Result<(), EmailError> {
        if self.to.is_empty() || self.to.iter().any(|address| address.trim().is_empty()) {
            return Err(EmailError::EmptyRecipients);
        }

        if self.subject.trim().is_empty() {
            return Err(EmailError::EmptySubject);
        }

        if self
            .html
            .as_deref()
            .is_none_or(|body| body.trim().is_empty())
            && self
                .text
                .as_deref()
                .is_none_or(|body| body.trim().is_empty())
        {
            return Err(EmailError::EmptyBody);
        }

        Ok(())
    }
}

impl fmt::Display for EmailError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRecipients => write!(formatter, "email must include at least one recipient"),
            Self::EmptySubject => write!(formatter, "email subject must not be empty"),
            Self::EmptyBody => write!(formatter, "email must include html or text content"),
            Self::RateLimited => write!(formatter, "email send was rate limited"),
            Self::Request(error) => write!(formatter, "email request failed: {error}"),
            Self::ProxyResponse { status, body } => {
                write!(formatter, "email proxy returned {status}: {body}")
            }
        }
    }
}

impl Error for EmailError {}

pub mod templates {
    pub fn registration_html(name: &str, email_verified: bool) -> String {
        let title = if email_verified {
            "You're in!"
        } else {
            "You're in - one tiny note"
        };
        let body = if email_verified {
            format!("Hi {name}, your account is ready. Time to start shaping something fun.")
        } else {
            format!(
                "Hi {name}, your account is ready. Your email still needs platform verification, so check the sign-in flow when you have a minute."
            )
        };

        casual_html(title, &body)
    }

    pub fn password_reset_html(login_url: &str) -> String {
        let title = escape_html("Let's get you back in");
        let body = escape_html(
            "Use the secure platform sign-in link below to continue. If you did not ask for this, you can ignore this email.",
        );
        let login_url = escape_html(login_url);

        format!(
            r#"<!doctype html>
<html>
  <body style="font-family: Arial, sans-serif; color: #1f2937; line-height: 1.5;">
    <h1 style="font-size: 20px;">{title}</h1>
    <p>{body}</p>
    <p><a href="{login_url}" style="color: #0f766e; font-weight: bold;">Continue to sign in</a></p>
  </body>
</html>"#
        )
    }

    pub fn casual_html(title: &str, body: &str) -> String {
        let title = escape_html(title);
        let body = escape_html(body);

        format!(
            r#"<!doctype html>
<html>
  <body style="font-family: Arial, sans-serif; color: #1f2937; line-height: 1.5;">
    <h1 style="font-size: 20px;">{title}</h1>
    <p>{body}</p>
  </body>
</html>"#
        )
    }

    pub fn event_invitation_html(
        event_title: &str,
        inviter_name: &str,
        invite_url: &str,
        message: Option<&str>,
    ) -> String {
        let event_title = escape_html(event_title);
        let inviter_name = escape_html(inviter_name);
        let invite_url = escape_html(invite_url);
        let message_html = message
            .filter(|message| !message.trim().is_empty())
            .map(|message| format!("<p>{}</p>", escape_html(message.trim())))
            .unwrap_or_default();

        format!(
            r#"<!doctype html>
<html>
  <body style="font-family: Arial, sans-serif; color: #1f2937; line-height: 1.5;">
    <h1 style="font-size: 20px;">You're invited to {event_title}</h1>
    <p>{inviter_name} invited you to join this gathering.</p>
    {message_html}
    <p><a href="{invite_url}" style="color: #0f766e; font-weight: bold;">Accept or decline the invitation</a></p>
    <p style="color: #6b7280; font-size: 13px;">This link takes you to the event invitation page.</p>
  </body>
</html>"#
        )
    }

    pub fn event_invitation_text(
        event_title: &str,
        inviter_name: &str,
        invite_url: &str,
        message: Option<&str>,
    ) -> String {
        let mut body = format!(
            "{inviter_name} invited you to {event_title}.\n\nAccept or decline here: {invite_url}"
        );

        if let Some(message) = message.filter(|message| !message.trim().is_empty()) {
            body.push_str("\n\nMessage from the organizer:\n");
            body.push_str(message.trim());
        }

        body
    }

    pub fn rsvp_confirmation_html(
        event_title: &str,
        response_label: &str,
        note: Option<&str>,
    ) -> String {
        let event_title = escape_html(event_title);
        let response_label = escape_html(response_label);
        let note_html = note
            .filter(|note| !note.trim().is_empty())
            .map(|note| format!("<p>Your note: {}</p>", escape_html(note.trim())))
            .unwrap_or_default();

        format!(
            r#"<!doctype html>
<html>
  <body style="font-family: Arial, sans-serif; color: #1f2937; line-height: 1.5;">
    <h1 style="font-size: 20px;">RSVP received</h1>
    <p>Your RSVP for {event_title} is now <strong>{response_label}</strong>.</p>
    {note_html}
    <p style="color: #6b7280; font-size: 13px;">You can update your RSVP from the event invitation page.</p>
  </body>
</html>"#
        )
    }

    pub fn rsvp_confirmation_text(
        event_title: &str,
        response_label: &str,
        note: Option<&str>,
    ) -> String {
        let mut body = format!("Your RSVP for {event_title} is now {response_label}.");

        if let Some(note) = note.filter(|note| !note.trim().is_empty()) {
            body.push_str("\n\nYour note:\n");
            body.push_str(note.trim());
        }

        body
    }

    pub fn event_reminder_html(
        event_title: &str,
        recipient_name: &str,
        starts_at: &str,
        event_url: &str,
    ) -> String {
        let event_title = escape_html(event_title);
        let recipient_name = escape_html(recipient_name);
        let starts_at = escape_html(starts_at);
        let event_url = escape_html(event_url);

        format!(
            r#"<!doctype html>
<html>
  <body style="font-family: Arial, sans-serif; color: #1f2937; line-height: 1.5;">
    <h1 style="font-size: 20px;">Your event is coming up</h1>
    <p>Hi {recipient_name}, {event_title} is scheduled for {starts_at}.</p>
    <p><a href="{event_url}" style="color: #0f766e; font-weight: bold;">Open the event details</a></p>
    <p style="color: #6b7280; font-size: 13px;">A quick reminder so you have the details close by.</p>
  </body>
</html>"#
        )
    }

    pub fn event_reminder_text(
        event_title: &str,
        recipient_name: &str,
        starts_at: &str,
        event_url: &str,
    ) -> String {
        format!(
            "Hi {recipient_name}, {event_title} is scheduled for {starts_at}.\n\nOpen the event details: {event_url}"
        )
    }

    fn escape_html(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

#[cfg(test)]
mod tests {
    use super::{templates, EmailError, EmailMessage};

    #[test]
    fn message_requires_body() {
        let message = EmailMessage::new("person@example.com", "Hello");

        assert!(matches!(message.validate(), Err(EmailError::EmptyBody)));
    }

    #[test]
    fn casual_template_escapes_content() {
        let html = templates::casual_html("<Welcome>", "One & two");

        assert!(html.contains("&lt;Welcome&gt;"));
        assert!(html.contains("One &amp; two"));
    }

    #[test]
    fn invitation_template_escapes_copy_and_links_response() {
        let html = templates::event_invitation_html(
            "<Launch>",
            "Alex & Sam",
            "https://example.com/invite/token",
            Some("Bring <notes>"),
        );

        assert!(html.contains("&lt;Launch&gt;"));
        assert!(html.contains("Alex &amp; Sam"));
        assert!(html.contains("Bring &lt;notes&gt;"));
        assert!(html.contains("https://example.com/invite/token"));
    }

    #[test]
    fn rsvp_confirmation_template_escapes_note() {
        let html = templates::rsvp_confirmation_html("<Launch>", "Yes", Some("See <you> there"));

        assert!(html.contains("&lt;Launch&gt;"));
        assert!(html.contains("See &lt;you&gt; there"));
    }

    #[test]
    fn reminder_template_escapes_event_details() {
        let html = templates::event_reminder_html(
            "<Launch>",
            "Alex & Sam",
            "Jul 18 at <noon>",
            "https://example.com/events/1?x=<y>",
        );

        assert!(html.contains("&lt;Launch&gt;"));
        assert!(html.contains("Alex &amp; Sam"));
        assert!(html.contains("Jul 18 at &lt;noon&gt;"));
        assert!(html.contains("&lt;y&gt;"));
    }

    #[test]
    fn registration_template_uses_warm_copy() {
        let html = templates::registration_html("Alex", true);

        assert!(html.contains("You&#39;re in!"));
        assert!(html.contains("Hi Alex"));
    }

    #[test]
    fn password_reset_template_escapes_link() {
        let html = templates::password_reset_html("https://example.test/?next=<dash>");

        assert!(html.contains("Let&#39;s get you back in"));
        assert!(html.contains("&lt;dash&gt;"));
    }
}
