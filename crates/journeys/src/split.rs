use uuid::Uuid;

use crate::model::SplitBranch;

/// Deterministic weighted branch choice — the same `(profile_id, node_id)`
/// always yields the same branch, across process restarts, so a resumed
/// run picks up in the branch it already committed to. Uses a self-
/// contained FNV-1a hash rather than `std`'s `DefaultHasher` to avoid any
/// dependency on unspecified hasher-seeding behavior.
pub fn choose_branch(branches: &[SplitBranch], profile_id: Uuid, node_id: &str) -> String {
    let key = format!("{profile_id}:{node_id}");
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in key.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let total: u32 = branches.iter().map(|b| b.weight).sum();
    if total == 0 {
        return branches[0].next.clone();
    }
    let point = (hash % total as u64) as u32;

    let mut acc = 0u32;
    for b in branches {
        acc += b.weight;
        if point < acc {
            return b.next.clone();
        }
    }
    branches
        .last()
        .expect("branches is non-empty when total > 0")
        .next
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branches() -> Vec<SplitBranch> {
        vec![
            SplitBranch {
                next: "a".into(),
                weight: 50,
            },
            SplitBranch {
                next: "b".into(),
                weight: 50,
            },
        ]
    }

    #[test]
    fn same_profile_and_node_always_picks_same_branch() {
        let profile_id = Uuid::new_v4();
        let first = choose_branch(&branches(), profile_id, "split1");
        for _ in 0..20 {
            assert_eq!(choose_branch(&branches(), profile_id, "split1"), first);
        }
    }

    #[test]
    fn different_profiles_can_land_in_different_branches() {
        let node_id = "split1";
        let outcomes: std::collections::HashSet<String> = (0..50)
            .map(|_| choose_branch(&branches(), Uuid::new_v4(), node_id))
            .collect();
        assert!(outcomes.contains("a") || outcomes.contains("b"));
        assert!(outcomes.len() <= 2);
    }

    #[test]
    fn single_branch_always_wins() {
        let only = vec![SplitBranch {
            next: "only".into(),
            weight: 100,
        }];
        assert_eq!(choose_branch(&only, Uuid::new_v4(), "n"), "only");
    }

    #[test]
    fn distribution_is_roughly_even_over_many_profiles() {
        let mut count_a = 0;
        let n = 2000;
        for _ in 0..n {
            if choose_branch(&branches(), Uuid::new_v4(), "split1") == "a" {
                count_a += 1;
            }
        }
        let ratio = count_a as f64 / n as f64;
        assert!((0.4..0.6).contains(&ratio), "ratio was {ratio}");
    }
}
