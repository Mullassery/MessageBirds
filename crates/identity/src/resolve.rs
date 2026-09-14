use mb_core::IdentityRef;
use uuid::Uuid;

/// Pure decision logic for identity resolution, kept separate from the
/// database I/O so it can be tested directly.
///
/// `existing[i]` is the profile id already linked to `claims[i]`, if any
/// (from a prior lookup against the identity graph). When claims disagree
/// — pointing at more than one existing profile — the claim marked
/// `primary` wins; otherwise the first claim with a match wins. Returns
/// `None` when none of the claims have ever been seen before, meaning the
/// caller should mint a new profile id.
pub fn pick_canonical(claims: &[IdentityRef], existing: &[Option<Uuid>]) -> Option<Uuid> {
    debug_assert_eq!(claims.len(), existing.len());

    if let Some(id) = claims
        .iter()
        .zip(existing)
        .find(|(c, e)| c.primary && e.is_some())
        .and_then(|(_, e)| *e)
    {
        return Some(id);
    }

    existing.iter().flatten().next().copied()
}

/// Every distinct existing profile id other than the chosen canonical one
/// — these get merged into the canonical profile.
pub fn profiles_to_merge(canonical: Uuid, existing: &[Option<Uuid>]) -> Vec<Uuid> {
    let mut others: Vec<Uuid> = existing
        .iter()
        .flatten()
        .copied()
        .filter(|id| *id != canonical)
        .collect();
    others.sort();
    others.dedup();
    others
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(namespace: &str, primary: bool) -> IdentityRef {
        IdentityRef {
            namespace: namespace.into(),
            value: "v".into(),
            primary,
            source: "test".into(),
            confidence: 1.0,
        }
    }

    #[test]
    fn no_existing_matches_returns_none() {
        let claims = vec![claim("anonymous_id", false)];
        assert_eq!(pick_canonical(&claims, &[None]), None);
    }

    #[test]
    fn single_match_is_canonical() {
        let id = Uuid::new_v4();
        let claims = vec![claim("anonymous_id", false)];
        assert_eq!(pick_canonical(&claims, &[Some(id)]), Some(id));
    }

    #[test]
    fn primary_claim_wins_over_conflicting_matches() {
        let anon_profile = Uuid::new_v4();
        let email_profile = Uuid::new_v4();
        let claims = vec![claim("anonymous_id", false), claim("email", true)];
        let existing = [Some(anon_profile), Some(email_profile)];
        assert_eq!(pick_canonical(&claims, &existing), Some(email_profile));
    }

    #[test]
    fn merge_set_excludes_canonical() {
        let canonical = Uuid::new_v4();
        let other = Uuid::new_v4();
        let existing = [Some(canonical), Some(other), Some(canonical)];
        assert_eq!(profiles_to_merge(canonical, &existing), vec![other]);
    }
}
