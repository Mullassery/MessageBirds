use thiserror::Error;

use crate::model::{FieldCandidate, MergePolicy, Strategy};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MergePolicyError {
    #[error("merge strategy {0:?} is not implemented in this phase")]
    NotImplemented(Strategy),
}

/// Picks the winning candidate for a single profile field under a policy.
/// Returns `None` if there are no candidates at all.
pub fn apply_policy<'a>(
    policy: &MergePolicy,
    candidates: &'a [FieldCandidate],
) -> Result<Option<&'a FieldCandidate>, MergePolicyError> {
    if !policy.strategy.is_implemented() {
        return Err(MergePolicyError::NotImplemented(policy.strategy));
    }
    if candidates.is_empty() {
        return Ok(None);
    }

    match policy.strategy {
        Strategy::LatestTimestamp => Ok(latest(candidates)),
        Strategy::SourcePriority => {
            let sources: Vec<String> = policy
                .config
                .get("sources")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Ok(source_priority(candidates, &sources))
        }
        _ => unreachable!("guarded by is_implemented() above"),
    }
}

fn latest(candidates: &[FieldCandidate]) -> Option<&FieldCandidate> {
    candidates.iter().max_by_key(|c| c.observed_at)
}

/// Lower index in `sources` = higher priority. A candidate whose source
/// isn't in the list is treated as lowest priority. Ties (including "no
/// priority list configured at all") fall back to latest-timestamp.
fn source_priority<'a>(
    candidates: &'a [FieldCandidate],
    sources: &[String],
) -> Option<&'a FieldCandidate> {
    let rank = |c: &FieldCandidate| -> usize {
        sources
            .iter()
            .position(|s| s == &c.source)
            .unwrap_or(sources.len())
    };

    candidates.iter().min_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then(b.observed_at.cmp(&a.observed_at))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn candidate(source: &str, value: &str, offset_secs: i64) -> FieldCandidate {
        FieldCandidate {
            source: source.to_string(),
            value: serde_json::json!(value),
            observed_at: Utc::now() + Duration::seconds(offset_secs),
        }
    }

    fn policy(strategy: Strategy, config: serde_json::Value) -> MergePolicy {
        MergePolicy {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "test".into(),
            strategy,
            config,
            version: 1,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn latest_timestamp_picks_most_recent() {
        let p = policy(Strategy::LatestTimestamp, serde_json::json!({}));
        let candidates = vec![candidate("crm", "old", -10), candidate("mobile", "new", 0)];
        let winner = apply_policy(&p, &candidates).unwrap().unwrap();
        assert_eq!(winner.value, serde_json::json!("new"));
    }

    #[test]
    fn source_priority_honors_configured_order() {
        let p = policy(
            Strategy::SourcePriority,
            serde_json::json!({"sources": ["crm", "mobile"]}),
        );
        let candidates = vec![
            candidate("mobile", "from_mobile", 0),
            candidate("crm", "from_crm", -100),
        ];
        let winner = apply_policy(&p, &candidates).unwrap().unwrap();
        assert_eq!(winner.value, serde_json::json!("from_crm"));
    }

    #[test]
    fn source_priority_falls_back_to_latest_when_unconfigured() {
        let p = policy(Strategy::SourcePriority, serde_json::json!({"sources": []}));
        let candidates = vec![candidate("crm", "old", -10), candidate("mobile", "new", 0)];
        let winner = apply_policy(&p, &candidates).unwrap().unwrap();
        assert_eq!(winner.value, serde_json::json!("new"));
    }

    #[test]
    fn unimplemented_strategy_errors_explicitly() {
        let p = policy(Strategy::ConfidenceWeighted, serde_json::json!({}));
        let candidates = vec![candidate("crm", "x", 0)];
        assert_eq!(
            apply_policy(&p, &candidates),
            Err(MergePolicyError::NotImplemented(
                Strategy::ConfidenceWeighted
            ))
        );
    }

    #[test]
    fn no_candidates_returns_none() {
        let p = policy(Strategy::LatestTimestamp, serde_json::json!({}));
        assert_eq!(apply_policy(&p, &[]).unwrap(), None);
    }
}
