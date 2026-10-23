//! Klaviyo profile-import destination.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::profile::{email, first_name, last_name, phone};
use crate::webhook::{ConnectorError, DestinationConnector};

const DEFAULT_BASE_URL: &str = "https://a.klaviyo.com";
const DEFAULT_REVISION: &str = "2024-10-15";

pub struct KlaviyoConnector {
    client: reqwest::Client,
}

impl KlaviyoConnector {
    pub fn new() -> Self {
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(15));
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

impl Default for KlaviyoConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DestinationConnector for KlaviyoConnector {
    async fn send(
        &self,
        config: &serde_json::Value,
        payload: &serde_json::Value,
    ) -> Result<(), ConnectorError> {
        let api_key = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
            .ok_or(ConnectorError::InvalidConfig("api_key"))?;
        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_BASE_URL)
            .trim_end_matches('/');
        let revision = config
            .get("revision")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_REVISION);
        let email = email(payload).ok_or(ConnectorError::InvalidPayload("email"))?;

        let mut attributes = serde_json::Map::new();
        attributes.insert("email".into(), json!(email));
        if let Some(value) = first_name(payload) {
            attributes.insert("first_name".into(), json!(value));
        }
        if let Some(value) = last_name(payload) {
            attributes.insert("last_name".into(), json!(value));
        }
        if let Some(value) = phone(payload) {
            attributes.insert("phone_number".into(), json!(value));
        }

        let url = pch_net_guard::validate_outbound_url(&format!("{base_url}/api/profile-import/"))
            .map_err(|e| ConnectorError::UnsafeUrl(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Klaviyo-API-Key {api_key}"))
            .header("revision", revision)
            .json(&json!({
                "data": {
                    "type": "profile",
                    "attributes": attributes,
                }
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ConnectorError::NonSuccessStatus(response.status().as_u16()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn missing_api_key_is_invalid_config() {
        let connector = KlaviyoConnector::new();
        let error = connector
            .send(&json!({}), &json!({"mixins": {}}))
            .await
            .unwrap_err();
        assert!(matches!(error, ConnectorError::InvalidConfig("api_key")));
    }

    #[tokio::test]
    async fn missing_email_is_invalid_payload() {
        let connector = KlaviyoConnector::new();
        let error = connector
            .send(&json!({"api_key": "key"}), &json!({"mixins": {}}))
            .await
            .unwrap_err();
        assert!(matches!(error, ConnectorError::InvalidPayload("email")));
    }
}
