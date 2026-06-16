use crate::config;

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModelRewrite {
    pub original: String,
    pub new_name: String,
}

/// Resolve the effective rewrite for `model` (most specific pattern wins)
/// Grant rules take precedences over provider rules (Still using most specific pattern wins logic)
pub fn resolve(
    model: &str,
    grant_rules: &[config::ModelRule],
    provider_rules: &[config::ModelRule],
) -> Option<ResolvedModelRewrite> {
    let rule = match crate::matching::most_specific_match(grant_rules, model, |r| &r.pattern) {
        Some(grant_rule) => grant_rule,
        None => crate::matching::most_specific_match(provider_rules, model, |r| &r.pattern)?,
    };
    let new_name = rule.rewrite.clone()?;
    Some(ResolvedModelRewrite {
        original: model.to_owned(),
        new_name,
    })
}

impl ResolvedModelRewrite {
    pub fn apply_to_path(&self, path: &str) -> String {
        let encoded = percent_encoding::utf8_percent_encode(
            &self.new_name,
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        path.replace(&self.original, &encoded)
    }
}

pub fn rewrite_request_path(
    mut request: axum::extract::Request,
    rewrite: &ResolvedModelRewrite,
) -> anyhow::Result<axum::extract::Request> {
    let pq = request.uri().path_and_query();
    let path = pq.map(|pq| pq.path()).unwrap_or("/");
    let query_suffix = pq
        .and_then(|pq| pq.query())
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    let new_path = rewrite.apply_to_path(path);

    let mut parts = request.uri().clone().into_parts();
    parts.path_and_query = Some(format!("{new_path}{query_suffix}").parse()?);
    *request.uri_mut() = http::Uri::from_parts(parts)?;
    Ok(request)
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
    fn resolved_model_rewrite_match_with_rewrite_rule() {
        let provider_rules = vec![model_rule("claude-*", Some("target"))];
        let resolved_model_rewrite = resolve("claude-opus", &[], &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.original, "claude-opus");
        assert_eq!(resolved_model_rewrite.new_name, "target");
    }

    #[test]
    fn no_resolved_model_rewrite_when_no_rewrite_rule() {
        let provider_rules = vec![model_rule("claude-*", None)];
        assert_eq!(resolve("claude-opus", &[], &provider_rules), None);
    }

    #[test]
    fn no_resolved_model_rewrite_when_no_matching_model() {
        let provider_rules = vec![model_rule("claude-*", Some("target"))];
        assert_eq!(resolve("gpt-4o", &[], &provider_rules), None);
    }

    #[test]
    fn resolved_model_rewrite_most_specific_win() {
        let provider_rules = vec![
            model_rule("no-match", Some("no-rewrite")),
            model_rule("claude-opus-*", Some("specific-claude")),
            model_rule("claude-*", Some("broad-claude")),
            model_rule("gemini-*", Some("broad-gemini")),
            model_rule("gemini-flash-*", Some("specific-gemini")),
        ];

        // The most specific matching pattern wins regardless of declaration order
        // (ordering itself is covered exhaustively in `matching`).
        let resolved_model_rewrite = resolve("claude-opus-4-5", &[], &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "specific-claude");

        let resolved_model_rewrite = resolve("gemini-flash-3", &[], &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "specific-gemini");
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
        let resolved_model_rewrite = resolve("claude-opus-4-8", &grant, &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "grant-claude");

        // Matching grant with no rewrite shadows the provider rewrite to None.
        assert_eq!(resolve("gemini-flash-3", &grant, &provider_rules), None);

        // No grant matches: the provider rule applies.
        let resolved_model_rewrite = resolve("gemini-pro", &grant, &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "provider-gemini");
    }

    #[test]
    fn apply_to_path_does_rewrite_and_encode() {
        let arn =
            "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/abcd1234567";
        let resolved_model_rewrite = ResolvedModelRewrite {
            original: "us.anthropic.claude-opus-4-5x".to_owned(),
            new_name: arn.to_owned(),
        };
        let out =
            resolved_model_rewrite.apply_to_path("/model/us.anthropic.claude-opus-4-5x/invoke");
        assert!(!out.contains("us.anthropic.claude-opus-4-5x"));

        // `:` and `/` and `-` are all encoded.
        assert_eq!(arn.matches(":").count(), out.matches("%3A").count());
        assert_eq!(arn.matches("/").count(), out.matches("%2F").count());
        assert_eq!(arn.matches("-").count(), out.matches("%2D").count());
    }

    #[test]
    fn apply_to_path_is_noop_when_original_absent() {
        let resolved_model_rewrite = ResolvedModelRewrite {
            original: "not-in-path".to_owned(),
            new_name: "should-not-rewrite".to_owned(),
        };
        let path = "/model/some-other-model/invoke";
        assert_eq!(resolved_model_rewrite.apply_to_path(path), path);
    }
}
