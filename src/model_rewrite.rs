use crate::config;

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModelRewrite {
    pub original: String,
    pub new_name: String,
}

/// Resolve the effective rewrite for `model` (first matching rule).
/// Per-user grant rules override provider rewrite rules.
pub fn resolve(
    model: &str,
    grant_rules: &[config::ModelRule],
    provider_rules: &[config::ModelRule],
) -> Option<ResolvedModelRewrite> {
    let grant_keys: std::collections::HashSet<&str> =
        grant_rules.iter().map(|r| r.pattern.as_str()).collect();
    let rule = grant_rules
        .iter()
        .chain(
            provider_rules
                .iter()
                .filter(|r| !grant_keys.contains(r.pattern.as_str())),
        )
        .find(|r| r.pattern.matches(model))?;
    let resolved_model_rewrite = rule.rewrite.clone()?;
    Some(ResolvedModelRewrite {
        original: model.to_owned(),
        new_name: resolved_model_rewrite,
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
    fn resolved_model_rewrite_first_match_win() {
        let provider_rules = vec![
            model_rule("no-match", Some("no-rewrite")),
            model_rule("claude-opus-*", Some("specific-claude")),
            model_rule("claude-*", Some("broad-claude")),
            model_rule("gemini-*", Some("broad-gemini")),
            model_rule("gemini-flash-*", Some("specific-gemini")),
        ];

        // Only the order matters, the specificity doesn't have any impact
        let resolved_model_rewrite = resolve("claude-opus-4-5", &[], &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "specific-claude");

        let resolved_model_rewrite = resolve("gemini-flash-3", &[], &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "broad-gemini");
    }

    #[test]
    fn resolved_model_rewrite_no_rewrite_short_circuit() {
        // A no-rewrite pattern declared first short-circuits to None
        // even though a later pattern would rewrite.
        let provider_rules = vec![
            model_rule("claude-opus*", None),
            model_rule("claude-*", Some("later")),
        ];
        assert_eq!(resolve("claude-opus-4-5", &[], &provider_rules), None);
    }

    #[test]
    fn grant_overrides_provider_rewrite_rule_for_same_key() {
        let provider_rules = vec![
            model_rule("model-1-*", Some("provider-1")),
            model_rule("model-2-*", Some("provider-2")),
        ];
        let grant = vec![
            model_rule("model-1-*", Some("grant")),
            model_rule("model-2-*", None),
        ];

        // Grant update the rewrite rule
        let resolved_model_rewrite = resolve("model-1-1", &grant, &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "grant");

        // Grant override the rewrite rule (Disabling it)
        assert_eq!(resolve("model-2-2", &grant, &provider_rules), None);
    }

    #[test]
    fn grant_rewrite_rules_are_evaluated_before_provider_rewrite_rules() {
        let provider_rules = vec![
            model_rule("claude-*", Some("provider-generic")),
            model_rule("openai-gpt-5*", Some("provider-specific")),
            model_rule("gemini-*", Some("provider")),
        ];
        let grant = vec![
            model_rule("claude-opus-*", Some("grant-specific")),
            model_rule("openai-*", Some("grant-generic")),
            model_rule("gemini-flash-*", None),
        ];
        let resolved_model_rewrite = resolve("claude-opus-4.6", &grant, &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "grant-specific");

        let resolved_model_rewrite = resolve("openai-gpt-5.5", &grant, &provider_rules).unwrap();
        assert_eq!(resolved_model_rewrite.new_name, "grant-generic");

        assert_eq!(resolve("gemini-flash-3", &grant, &provider_rules), None);
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
