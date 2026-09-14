use strsim::jaro_winkler;

/// The non-identity signals used to score whether two profiles might be
/// the same customer. Deliberately not identity claims — those are
/// resolved deterministically (`mb-identity`) and hashed, so they can't be
/// fuzzy-compared. This is a suggestion, never an automatic merge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PersonSignal {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub device_id: Option<String>,
}

/// Confidence that `a` and `b` are the same customer, in `[0.0, 1.0]`.
/// Weighted 50/50 between a shared device id and name similarity; either
/// signal being absent on either side contributes 0 for that half rather
/// than being skipped, so a bare device match alone caps at 0.5 — not
/// enough to imply a name match nobody has actually seen.
pub fn score(a: &PersonSignal, b: &PersonSignal) -> f64 {
    let device_score = match (&a.device_id, &b.device_id) {
        (Some(x), Some(y)) if x == y => 1.0,
        _ => 0.0,
    };

    let name_score = name_similarity(a, b);

    0.5 * device_score + 0.5 * name_score
}

fn name_similarity(a: &PersonSignal, b: &PersonSignal) -> f64 {
    let pairs = [(&a.first_name, &b.first_name), (&a.last_name, &b.last_name)];

    let scores: Vec<f64> = pairs
        .into_iter()
        .filter_map(|(x, y)| match (x, y) {
            (Some(x), Some(y)) if !x.is_empty() && !y.is_empty() => {
                Some(jaro_winkler(&x.to_lowercase(), &y.to_lowercase()))
            }
            _ => None,
        })
        .collect();

    if scores.is_empty() {
        return 0.0;
    }
    scores.iter().sum::<f64>() / scores.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_device_and_name_scores_high() {
        let a = PersonSignal {
            first_name: Some("Jane".into()),
            last_name: Some("Doe".into()),
            device_id: Some("device-1".into()),
        };
        let b = a.clone();
        assert_eq!(score(&a, &b), 1.0);
    }

    #[test]
    fn shared_device_alone_caps_at_half() {
        let a = PersonSignal {
            first_name: None,
            last_name: None,
            device_id: Some("device-1".into()),
        };
        let b = PersonSignal {
            first_name: None,
            last_name: None,
            device_id: Some("device-1".into()),
        };
        assert_eq!(score(&a, &b), 0.5);
    }

    #[test]
    fn no_shared_signals_scores_zero() {
        let a = PersonSignal {
            first_name: Some("Jane".into()),
            last_name: Some("Doe".into()),
            device_id: Some("device-1".into()),
        };
        let b = PersonSignal {
            first_name: Some("Bob".into()),
            last_name: Some("Smith".into()),
            device_id: Some("device-2".into()),
        };
        assert!(score(&a, &b) < 0.3);
    }

    #[test]
    fn similar_names_score_higher_than_dissimilar() {
        let base = PersonSignal {
            first_name: Some("Jonathan".into()),
            last_name: None,
            device_id: None,
        };
        let close = PersonSignal {
            first_name: Some("Jonathon".into()),
            last_name: None,
            device_id: None,
        };
        let far = PersonSignal {
            first_name: Some("Zephyr".into()),
            last_name: None,
            device_id: None,
        };
        assert!(score(&base, &close) > score(&base, &far));
    }
}
