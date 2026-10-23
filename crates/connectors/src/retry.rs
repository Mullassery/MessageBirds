//! Delivery policy for outbound destination connectors.
//!
//! Connector adapters deliberately stay small: they build one request and
//! report one result. Retries, backoff, and pacing are cross-cutting and
//! belong here, so every destination gets the same behavior.
//!
//! Configuration is optional and lives in the destination `config` JSON:
//!
//! ```json
//! {
//!   "retry": {
//!     "max_attempts": 3,
//!     "initial_backoff_ms": 250,
//!     "max_backoff_ms": 5000
//!   },
//!   "rate_limit_per_second": 10
//! }
//! ```
//!
//! Retries are limited to transient failures: transport errors, timeouts,
//! HTTP 408/425/429, and 5xx responses. Validation/config/auth failures are
//! never retried.

use std::time::Duration;

use serde_json::Value;

use crate::webhook::{ConnectorError, DestinationConnector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(250),
            max_backoff: Duration::from_secs(5),
        }
    }
}

impl RetryPolicy {
    pub fn from_config(config: &Value) -> Self {
        let default = Self::default();
        let Some(retry) = config.get("retry").and_then(Value::as_object) else {
            return default;
        };

        let max_attempts = retry
            .get("max_attempts")
            .and_then(Value::as_u64)
            .map(|value| value.clamp(1, 10) as u32)
            .unwrap_or(default.max_attempts);

        let initial_backoff = Duration::from_millis(
            retry
                .get("initial_backoff_ms")
                .and_then(Value::as_u64)
                .unwrap_or(default.initial_backoff.as_millis() as u64)
                .min(60_000),
        );
        let max_backoff = Duration::from_millis(
            retry
                .get("max_backoff_ms")
                .and_then(Value::as_u64)
                .unwrap_or(default.max_backoff.as_millis() as u64)
                .min(120_000),
        )
        .max(initial_backoff);

        Self {
            max_attempts,
            initial_backoff,
            max_backoff,
        }
    }

    /// Backoff after the given zero-based failed attempt.
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let multiplier = 1u32.checked_shl(attempt.min(31)).unwrap_or(u32::MAX);
        self.initial_backoff
            .saturating_mul(multiplier)
            .min(self.max_backoff)
    }
}

pub fn is_retryable(error: &ConnectorError) -> bool {
    match error {
        ConnectorError::Delivery(_) => true,
        ConnectorError::NonSuccessStatus(status) => {
            matches!(*status, 408 | 425 | 429 | 500..=599)
        }
        ConnectorError::InvalidConfig(_)
        | ConnectorError::InvalidPayload(_)
        | ConnectorError::UnsafeUrl(_)
        | ConnectorError::Serialization(_) => false,
    }
}

/// Sends one payload with the destination's configured retry policy.
pub async fn send_with_retry(
    connector: &dyn DestinationConnector,
    config: &Value,
    payload: &Value,
) -> Result<(), ConnectorError> {
    let policy = RetryPolicy::from_config(config);
    let mut last_error = None;

    for attempt in 0..policy.max_attempts {
        match connector.send(config, payload).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                let has_retry = attempt + 1 < policy.max_attempts && is_retryable(&error);
                if !has_retry {
                    return Err(error);
                }
                tokio::time::sleep(policy.delay_for(attempt)).await;
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or(ConnectorError::InvalidConfig("retry policy")))
}

/// Minimum interval between outbound sends, derived from
/// `rate_limit_per_second`. `None` means unthrottled.
pub fn rate_limit_interval(config: &Value) -> Option<Duration> {
    let per_second = config
        .get("rate_limit_per_second")
        .and_then(Value::as_f64)
        .filter(|value| *value > 0.0)?;
    Some(Duration::from_secs_f64(1.0 / per_second))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockConnector {
        calls: AtomicUsize,
        failures: usize,
        status: u16,
    }

    impl MockConnector {
        fn new(failures: usize, status: u16) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                failures,
                status,
            }
        }
    }

    #[async_trait]
    impl DestinationConnector for MockConnector {
        async fn send(&self, _config: &Value, _payload: &Value) -> Result<(), ConnectorError> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call < self.failures {
                Err(ConnectorError::NonSuccessStatus(self.status))
            } else {
                Ok(())
            }
        }
    }

    fn no_delay_config(max_attempts: u32) -> Value {
        json!({
            "retry": {
                "max_attempts": max_attempts,
                "initial_backoff_ms": 0,
                "max_backoff_ms": 0
            }
        })
    }

    #[test]
    fn defaults_are_three_attempts_with_exponential_backoff() {
        let policy = RetryPolicy::from_config(&json!({}));
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.delay_for(0), Duration::from_millis(250));
        assert_eq!(policy.delay_for(1), Duration::from_millis(500));
        assert_eq!(policy.delay_for(2), Duration::from_millis(1_000));
    }

    #[test]
    fn config_is_clamped_and_capped() {
        let policy = RetryPolicy::from_config(&json!({
            "retry": {
                "max_attempts": 99,
                "initial_backoff_ms": 100,
                "max_backoff_ms": 500
            }
        }));
        assert_eq!(policy.max_attempts, 10);
        assert_eq!(policy.delay_for(0), Duration::from_millis(100));
        assert_eq!(policy.delay_for(10), Duration::from_millis(500));
    }

    #[test]
    fn max_backoff_never_falls_below_initial() {
        let policy = RetryPolicy::from_config(&json!({
            "retry": {"initial_backoff_ms": 2_000, "max_backoff_ms": 10}
        }));
        assert_eq!(policy.max_backoff, Duration::from_millis(2_000));
    }

    #[test]
    fn transient_failures_are_retryable() {
        assert!(is_retryable(&ConnectorError::NonSuccessStatus(429)));
        assert!(is_retryable(&ConnectorError::NonSuccessStatus(503)));
        assert!(is_retryable(&ConnectorError::NonSuccessStatus(408)));
        assert!(!is_retryable(&ConnectorError::NonSuccessStatus(400)));
        assert!(!is_retryable(&ConnectorError::InvalidPayload("email")));
        assert!(!is_retryable(&ConnectorError::UnsafeUrl("private".into())));
    }

    #[test]
    fn rate_limit_is_optional_and_positive_only() {
        assert_eq!(rate_limit_interval(&json!({})), None);
        assert_eq!(
            rate_limit_interval(&json!({"rate_limit_per_second": 0})),
            None
        );
        assert_eq!(
            rate_limit_interval(&json!({"rate_limit_per_second": 2})),
            Some(Duration::from_millis(500))
        );
    }

    #[tokio::test]
    async fn retries_transient_failures_then_succeeds() {
        let connector = MockConnector::new(2, 503);
        let result = send_with_retry(
            &connector,
            &no_delay_config(3),
            &json!({"profile_id": "p1"}),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(connector.calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stops_after_max_attempts() {
        let connector = MockConnector::new(99, 429);
        let result = send_with_retry(
            &connector,
            &no_delay_config(3),
            &json!({"profile_id": "p1"}),
        )
        .await;
        assert!(matches!(result, Err(ConnectorError::NonSuccessStatus(429))));
        assert_eq!(connector.calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn does_not_retry_permanent_failures() {
        let connector = MockConnector::new(99, 400);
        let result = send_with_retry(
            &connector,
            &no_delay_config(3),
            &json!({"profile_id": "p1"}),
        )
        .await;
        assert!(matches!(result, Err(ConnectorError::NonSuccessStatus(400))));
        assert_eq!(connector.calls.load(Ordering::SeqCst), 1);
    }
}
