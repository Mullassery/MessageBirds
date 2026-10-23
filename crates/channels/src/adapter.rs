use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use crate::model::RenderedMessage;

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("channel config is missing or has an invalid '{0}' field")]
    InvalidConfig(&'static str),
    #[error("channel url rejected by outbound network policy: {0}")]
    UnsafeUrl(String),
    #[error("delivery failed: {0}")]
    Delivery(#[from] reqwest::Error),
    #[error("channel responded with status {0}")]
    NonSuccessStatus(u16),
}

/// A transport that can deliver a rendered message (Section 28). Real
/// providers (SES, Twilio, FCM, ...) would each implement this the same
/// way `WebhookChannelAdapter` does — only `send`'s body differs.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    async fn send(
        &self,
        config: &serde_json::Value,
        message: &RenderedMessage,
    ) -> Result<(), ChannelError>;
}

pub struct WebhookChannelAdapter {
    client: reqwest::Client,
}

impl WebhookChannelAdapter {
    pub fn new() -> Self {
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(10));
        // Tenant-configured URLs are untrusted; refuse private/loopback/
        // link-local targets unless explicitly opted out.
        let builder = if pch_net_guard::guard_enabled() {
            pch_net_guard::harden_client(builder)
        } else {
            builder
        };
        Self {
            client: builder
                .build()
                .expect("reqwest client builds with a fixed, valid configuration"),
        }
    }
}

impl Default for WebhookChannelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelAdapter for WebhookChannelAdapter {
    async fn send(
        &self,
        config: &serde_json::Value,
        message: &RenderedMessage,
    ) -> Result<(), ChannelError> {
        let raw_url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(ChannelError::InvalidConfig("url"))?;
        let url = pch_net_guard::validate_outbound_url(raw_url)
            .map_err(|e| ChannelError::UnsafeUrl(e.to_string()))?;

        let response = self.client.post(url).json(message).send().await?;

        if !response.status().is_success() {
            return Err(ChannelError::NonSuccessStatus(response.status().as_u16()));
        }
        Ok(())
    }
}
