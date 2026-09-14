use mb_merge_policy::{apply_policy, FieldCandidate, MergePolicy, MergePolicyError};

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// The existing value already wins under the policy; nothing to write.
    Unchanged,
    /// The incoming value wins and should be persisted.
    Updated(FieldCandidate),
}

pub fn decide(
    policy: &MergePolicy,
    existing: Option<FieldCandidate>,
    incoming: FieldCandidate,
) -> Result<Decision, MergePolicyError> {
    let Some(existing) = existing else {
        return Ok(Decision::Updated(incoming));
    };

    let candidates = vec![existing.clone(), incoming.clone()];
    let winner = apply_policy(policy, &candidates)?.expect("candidates is non-empty");

    if *winner == existing {
        Ok(Decision::Unchanged)
    } else {
        Ok(Decision::Updated(incoming))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use mb_merge_policy::Strategy;
    use uuid::Uuid;

    fn policy() -> MergePolicy {
        MergePolicy {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "test".into(),
            strategy: Strategy::LatestTimestamp,
            config: serde_json::json!({}),
            version: 1,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn no_existing_value_always_updates() {
        let incoming = FieldCandidate {
            source: "web".into(),
            value: serde_json::json!("Jane"),
            observed_at: Utc::now(),
        };
        assert_eq!(
            decide(&policy(), None, incoming.clone()).unwrap(),
            Decision::Updated(incoming)
        );
    }

    #[test]
    fn newer_incoming_value_updates() {
        let existing = FieldCandidate {
            source: "crm".into(),
            value: serde_json::json!("old"),
            observed_at: Utc::now() - Duration::seconds(60),
        };
        let incoming = FieldCandidate {
            source: "web".into(),
            value: serde_json::json!("new"),
            observed_at: Utc::now(),
        };
        assert_eq!(
            decide(&policy(), Some(existing), incoming.clone()).unwrap(),
            Decision::Updated(incoming)
        );
    }

    #[test]
    fn older_incoming_value_is_unchanged() {
        let existing = FieldCandidate {
            source: "crm".into(),
            value: serde_json::json!("current"),
            observed_at: Utc::now(),
        };
        let incoming = FieldCandidate {
            source: "web".into(),
            value: serde_json::json!("stale"),
            observed_at: Utc::now() - Duration::seconds(60),
        };
        assert_eq!(
            decide(&policy(), Some(existing), incoming).unwrap(),
            Decision::Unchanged
        );
    }
}
