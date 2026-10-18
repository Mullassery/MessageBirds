//! HubSpot CRM destination (contacts batch upsert).

use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::profile::{email, first_name, last_name, phone};
use crate::webhook::{ConnectorError, DestinationConnector};

const DEFAULT_BASE_URL: &str = "https://api.hubapi.com";

pub struct HubspotConnector {
    client: reqwest::Client,
}

impl HubspotConnector {
    pub fn new() -> Self {
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(15));
        let builder = if mb_net_guard::guard_enabled() {
            mb_net_guard::harden_client(builder)
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

impl Default for HubspotConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DestinationConnector for HubspotConnector {
    async fn send(
        &self,
        config: &serde_json::Value,
        payload: &serde_json::Value,
    ) -> Result<(), ConnectorError> {
        let token = config
            .get("access_token")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
            .ok_or(ConnectorError::InvalidConfig("access_token"))?;
        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_BASE_URL)
            .trim_end_matches('/');
        let email = email(payload).ok_or(ConnectorError::InvalidPayload("email"))?;

        let mut properties = serde_json::Map::new();
        properties.insert("email".into(), json!(email));
        if let Some(value) = first_name(payload) {
            properties.insert("firstname".into(), json!(value));
        }
        if let Some(value) = last_name(payload) {
            properties.insert("lastname".into(), json!(value));
        }
        if let Some(value) = phone(payload) {
            properties.insert("phone".into(), json!(value));
        }

        let url = mb_net_guard::validate_outbound_url(&format!(
            "{base_url}/crm/v3/objects/contacts/batch/upsert"
        ))
        .map_err(|e| ConnectorError::UnsafeUrl(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(&json!({
                "inputs": [{
                    "idProperty": "email",
                    "id": email,
                    "properties": properties,
                }]
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
    async fn missing_token_is_invalid_config() {
        let connector = HubspotConnector::new();
        let error = connector
            .send(&json!({}), &json!({"mixins": {}}))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ConnectorError::InvalidConfig("access_token")
        ));
    }

    #[tokio::test]
    async fn missing_email_is_invalid_payload() {
        let connector = HubspotConnector::new();
        let error = connector
            .send(&json!({"access_token": "token"}), &json!({"mixins": {}}))
            .await
            .unwrap_err();
        assert!(matches!(error, ConnectorError::InvalidPayload("email")));
    }
}
