use std::time::Duration;

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("destination config is missing or has an invalid '{0}' field")]
    InvalidConfig(&'static str),
    #[error("destination payload is missing required field '{0}'")]
    InvalidPayload(&'static str),
    #[error("destination url rejected by outbound network policy: {0}")]
    UnsafeUrl(String),
    #[error("delivery failed: {0}")]
    Delivery(#[from] reqwest::Error),
    #[error("destination payload serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
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
        let raw_url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(ConnectorError::InvalidConfig("url"))?;
        let url = pch_net_guard::validate_outbound_url(raw_url)
            .map_err(|e| ConnectorError::UnsafeUrl(e.to_string()))?;

        // Sign the exact bytes that are transmitted, not a re-serialization
        // performed by the HTTP client. Delivery id + timestamp are part of
        // the signed input so a receiver can verify freshness and dedupe.
        let body = serde_json::to_vec(payload)?;
        let delivery_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();
        let signing_secret = config
            .get("signing_secret")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let mut request = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .header("x-pch-delivery", &delivery_id)
            .header("x-pch-timestamp", timestamp.to_string())
            .body(body.clone());

        if let Some(secret) = signing_secret {
            let signature = webhook_signature(secret, &delivery_id, timestamp, &body);
            request = request
                .header("x-pch-signature", format!("sha256={signature}"))
                .header("x-pch-signature-version", "v1");
        }

        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(ConnectorError::NonSuccessStatus(response.status().as_u16()));
        }
        Ok(())
    }
}

/// Sign `delivery_id.timestamp.body` with HMAC-SHA256.
///
/// Receivers must verify the exact raw request body, require a bounded clock
/// skew on `x-pch-timestamp`, and dedupe on `x-pch-delivery`.
pub fn webhook_signature(secret: &str, delivery_id: &str, timestamp: i64, body: &[u8]) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(delivery_id.as_bytes());
    mac.update(b".");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    fn receive_one_request(mut stream: std::net::TcpStream) -> (HashMap<String, String>, Vec<u8>) {
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        reader.read_line(&mut request_line).expect("request line");
        let mut headers = HashMap::new();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("header line");
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').expect("header colon");
            let name = name.trim().to_ascii_lowercase();
            if name == "content-length" {
                content_length = value.trim().parse().unwrap_or(0);
            }
            headers.insert(name, value.trim().to_string());
        }
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).expect("body");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .expect("response");
        (headers, body)
    }

    #[test]
    fn signature_covers_delivery_timestamp_and_body() {
        let body = br#"{"profile_id":"p1"}"#;
        let signature = webhook_signature("secret", "delivery-1", 1_700_000_000, body);
        assert_eq!(signature.len(), 64);

        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").unwrap();
        mac.update(b"delivery-1.1700000000.");
        mac.update(body);
        assert_eq!(signature, hex::encode(mac.finalize().into_bytes()));
    }

    #[test]
    fn signature_changes_with_any_signed_input() {
        let body = br#"{"profile_id":"p1"}"#;
        let base = webhook_signature("secret", "delivery-1", 1_700_000_000, body);
        assert_ne!(
            base,
            webhook_signature("secret", "delivery-2", 1_700_000_000, body)
        );
        assert_ne!(
            base,
            webhook_signature("secret", "delivery-1", 1_700_000_001, body)
        );
        assert_ne!(
            base,
            webhook_signature("secret", "delivery-1", 1_700_000_000, b"{}",)
        );
        assert_ne!(
            base,
            webhook_signature("other", "delivery-1", 1_700_000_000, body)
        );
    }

    #[tokio::test]
    async fn send_sets_verifiable_signing_headers_and_exact_body() {
        // The URL is deliberately loopback for the local receiver.
        std::env::set_var("PYCUSTOMERHUB_ALLOW_PRIVATE_WEBHOOKS", "1");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let receiver = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            receive_one_request(stream)
        });

        let connector = WebhookConnector {
            client: reqwest::Client::new(),
        };
        let config = serde_json::json!({
            "url": format!("http://{address}/hook"),
            "signing_secret": "secret"
        });
        let payload = serde_json::json!({"profile_id": "p1", "mixins": {}});
        connector.send(&config, &payload).await.unwrap();

        let (headers, body) = receiver.join().unwrap();
        assert_eq!(headers["x-pch-signature-version"], "v1");
        let delivery = &headers["x-pch-delivery"];
        let timestamp: i64 = headers["x-pch-timestamp"].parse().unwrap();
        let signature = headers["x-pch-signature"].strip_prefix("sha256=").unwrap();
        assert_eq!(
            signature,
            webhook_signature("secret", delivery, timestamp, &body)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            payload
        );
    }
}
