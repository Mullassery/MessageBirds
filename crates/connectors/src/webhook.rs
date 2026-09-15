use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("destination config is missing or has an invalid '{0}' field")]
    InvalidConfig(&'static str),
    #[error("delivery failed: {0}")]
    Delivery(#[from] reqwest::Error),
    #[error("destination responded with status {0}")]
    NonSuccessStatus(u16),
}

/// A place canonical profile data can be sent. Real destinations
/// (Salesforce, Braze, ad platforms, warehouses — Section 19) all
/// implement this the same way a webhook does; only `send`'s body differs.
#[async_trait]
pub trait DestinationConnector: Send + Sync {
    async fn send(
        &self,
        config: &serde_json::Value,
        payload: &serde_json::Value,
    ) -> Result<(), ConnectorError>;
}

pub struct WebhookConnector {
    client: reqwest::Client,
}

impl WebhookConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client builds with a fixed, valid configuration"),
        }
    }
}

impl Default for WebhookConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DestinationConnector for WebhookConnector {
    async fn send(
        &self,
        config: &serde_json::Value,
        payload: &serde_json::Value,
    ) -> Result<(), ConnectorError> {
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(ConnectorError::InvalidConfig("url"))?;

        let response = self.client.post(url).json(payload).send().await?;

        if !response.status().is_success() {
            return Err(ConnectorError::NonSuccessStatus(response.status().as_u16()));
        }
        Ok(())
    }
}
