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
}
