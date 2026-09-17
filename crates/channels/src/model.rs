use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const WEBHOOK_KIND: &str = "webhook";

/// A place a rendered message can be sent. `kind` is a label — `"email"`,
/// `"sms"`, `"push"`, `"whatsapp"` — but only the `webhook` *transport* is
/// implemented (`WebhookChannelAdapter`): a real SES/Twilio/etc. adapter
/// needs credentials this environment doesn't have, so for now every
/// channel kind is simulated by POSTing to a configured URL, the same
/// honest stand-in pattern as `mb-connectors`' one webhook destination.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Channel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// What actually gets sent — rendered by `mb-templates::render`, decoupled
/// from it here so `mb-channels` doesn't need to depend on that crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedMessage {
    pub subject: Option<String>,
    pub body: String,
}
