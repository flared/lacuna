use crate::config;

/// Resolve the rewrite target for `model` (most specific pattern wins).
/// Grant rules take precedence over provider rules (still using most specific pattern wins logic).
pub fn resolve(
    model: &str,
    grant_rules: &[config::ModelRule],
    provider_rules: &[config::ModelRule],
) -> Option<String> {
    let rule = match crate::matching::most_specific_match(grant_rules, model, |r| &r.pattern) {
        Some(grant_rule) => grant_rule,
        None => crate::matching::most_specific_match(provider_rules, model, |r| &r.pattern)?,
    };
    rule.rewrite.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_rule(pattern: &str, rewrite: Option<&str>) -> config::ModelRule {
        config::ModelRule {
            pattern: glob::Pattern::new(pattern).unwrap(),
            rewrite: rewrite.map(|r| r.to_owned()),
        }
    }

    #[test]
    fn resolve_match_with_rewrite_rule() {
        let provider_rules = vec![model_rule("claude-*", Some("target"))];
        assert_eq!(
            resolve("claude-opus", &[], &provider_rules),
            Some("target".to_owned())
        );
    }

    #[test]
    fn no_rewrite_when_no_rewrite_rule() {
        let provider_rules = vec![model_rule("claude-*", None)];
        assert_eq!(resolve("claude-opus", &[], &provider_rules), None);
    }

    #[test]
    fn no_rewrite_when_no_matching_model() {
        let provider_rules = vec![model_rule("claude-*", Some("target"))];
        assert_eq!(resolve("gpt-4o", &[], &provider_rules), None);
    }

    #[test]
    fn resolve_most_specific_win() {
        let provider_rules = vec![
            model_rule("no-match", Some("no-rewrite")),
            model_rule("claude-opus-*", Some("specific-claude")),
            model_rule("claude-*", Some("broad-claude")),
            model_rule("gemini-*", Some("broad-gemini")),
            model_rule("gemini-flash-*", Some("specific-gemini")),
        ];

        // The most specific matching pattern wins regardless of declaration order
        // (ordering itself is covered exhaustively in `matching`).
        assert_eq!(
            resolve("claude-opus-4-5", &[], &provider_rules),
            Some("specific-claude".to_owned())
        );
        assert_eq!(
            resolve("gemini-flash-3", &[], &provider_rules),
            Some("specific-gemini".to_owned())
        );
    }

    #[test]
    fn grant_tier_wins_over_provider() {
        // Any matching grant rule decides the tier outright; provider rules are
        // consulted only when no grant rule matches. This holds whether the grant
        // is more or less specific than the provider rule, and a matching grant
        // with no rewrite shadows the provider to `None`.
        let provider_rules = vec![
            model_rule("claude-opus-4*", Some("provider-specific")),
            model_rule("gemini-*", Some("provider-gemini")),
        ];
        let grant = vec![
            // Less specific than the provider rule it shadows.
            model_rule("claude-*", Some("grant-claude")),
            // No rewrite: shadows the provider rule to None.
            model_rule("gemini-flash-*", None),
        ];

        // Generic grant shadows the more-specific provider rewrite.
        assert_eq!(
            resolve("claude-opus-4-8", &grant, &provider_rules),
            Some("grant-claude".to_owned())
        );

        // Matching grant with no rewrite shadows the provider rewrite to None.
        assert_eq!(resolve("gemini-flash-3", &grant, &provider_rules), None);

        // No grant matches: the provider rule applies.
        assert_eq!(
            resolve("gemini-pro", &grant, &provider_rules),
            Some("provider-gemini".to_owned())
        );
    }
}
