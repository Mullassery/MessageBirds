//! Channel abstraction (Section 28): where a journey's rendered messages
//! actually go. One real transport, `webhook` — every channel kind
//! (`email`, `sms`, `push`, `whatsapp`, ...) is simulated by POSTing to a
//! configured URL, since real provider credentials (SES, Twilio, FCM, ...)
//! aren't available in this environment. Not a fake "looks like it sends"
//! shim — the webhook delivery genuinely happens and is verified against a
//! real listener.

mod adapter;
mod model;
mod repo;

pub use adapter::{ChannelAdapter, ChannelError, WebhookChannelAdapter};
pub use model::{Channel, RenderedMessage, WEBHOOK_KIND};
pub use repo::{ChannelRepo, ChannelRepoError, PgChannelRepo};
