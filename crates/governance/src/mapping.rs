/// Maps a Section 14 marketing action to the Section 17 consent purpose
/// that gates it. `None` means the action doesn't require consent — it's
/// treated as internal/operational rather than customer-facing marketing.
pub fn consent_purpose_for_action(action: &str) -> Option<&'static str> {
    match action {
        "EMAIL_MARKETING" => Some("email"),
        "SMS_MARKETING" => Some("sms"),
        "PUSH_MARKETING" => Some("push"),
        "WHATSAPP_MARKETING" => Some("whatsapp"),
        "PERSONALIZATION" => Some("personalization"),
        "ADVERTISING" | "CROSS_SITE_TARGETING" | "THIRD_PARTY_EXPORT" => Some("advertising"),
        "ANALYTICS" | "DATA_ENRICHMENT" | "AI_PROCESSING" => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketing_actions_map_to_purposes() {
        assert_eq!(consent_purpose_for_action("EMAIL_MARKETING"), Some("email"));
        assert_eq!(consent_purpose_for_action("SMS_MARKETING"), Some("sms"));
        assert_eq!(consent_purpose_for_action("PUSH_MARKETING"), Some("push"));
        assert_eq!(
            consent_purpose_for_action("WHATSAPP_MARKETING"),
            Some("whatsapp")
        );
        assert_eq!(
            consent_purpose_for_action("PERSONALIZATION"),
            Some("personalization")
        );
    }

    #[test]
    fn advertising_family_shares_one_purpose() {
        for action in ["ADVERTISING", "CROSS_SITE_TARGETING", "THIRD_PARTY_EXPORT"] {
            assert_eq!(consent_purpose_for_action(action), Some("advertising"));
        }
    }

    #[test]
    fn operational_actions_need_no_consent() {
        for action in ["ANALYTICS", "DATA_ENRICHMENT", "AI_PROCESSING", "UNKNOWN"] {
            assert_eq!(consent_purpose_for_action(action), None);
        }
    }
}
