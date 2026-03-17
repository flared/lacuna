use crate::request_metadata::RequestMetadata;

#[derive(Debug, Clone)]
pub struct Rule {
    pub providers: Vec<glob::Pattern>,
    pub models: Vec<glob::Pattern>,
}

#[derive(Debug, Clone, Default)]
pub struct Authorization {
    pub rules: Vec<Rule>,
}

impl Authorization {
    pub fn is_allowed(&self, request_metadata: &RequestMetadata) -> bool {
        let provider = &request_metadata.provider_key;
        let model = request_metadata.inspected.model.as_deref();
        self.rules.iter().any(|rule| {
            let provider_matches =
                rule.providers.is_empty() || rule.providers.iter().any(|p| p.matches(provider));
            let model_matches = rule.models.is_empty()
                || match model {
                    None => rule.models.iter().any(|p| p.matches("")),
                    Some(m) => rule.models.iter().any(|p| p.matches(m)),
                };
            provider_matches && model_matches
        })
    }

    pub fn deny_all() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_metadata::RequestInspectionMetadata;

    fn pattern(s: &str) -> glob::Pattern {
        glob::Pattern::new(s).unwrap()
    }

    fn rule(providers: &[&str], models: &[&str]) -> Rule {
        Rule {
            providers: providers.iter().map(|s| pattern(s)).collect(),
            models: models.iter().map(|s| pattern(s)).collect(),
        }
    }

    fn metadata(provider: &str, model: Option<&str>) -> RequestMetadata {
        RequestMetadata {
            provider_key: provider.to_string(),
            api_handler_id: String::new(),
            user_identity: None,
            user_agent: None,
            inspected: RequestInspectionMetadata {
                model: model.map(|m| m.to_string()),
            },
        }
    }

    #[test]
    fn test_is_allowed() {
        let auth = Authorization {
            rules: vec![
                rule(&["provider-a", "provider-a-*"], &["claude-*", "gpt-4o"]),
                rule(&["provider-b"], &["gpt-3.5-turbo"]),
                rule(&["*"], &["fallback"]),
            ],
        };

        // Provider with Some model (checks both provider and model)
        assert!(auth.is_allowed(&metadata("provider-a", Some("claude-sonnet-4-20250514"))));
        assert!(auth.is_allowed(&metadata("provider-a", Some("gpt-4o"))));
        assert!(!auth.is_allowed(&metadata("provider-a", Some("gpt-3.5-turbo"))));
        assert!(!auth.is_allowed(&metadata("provider-a", None)));

        // Wildcard provider with Some model
        assert!(auth.is_allowed(&metadata("provider-a-1", Some("claude-opus-4-20250514"))));

        // Auth can fallback on more permissive rule.
        assert!(auth.is_allowed(&metadata("provider-a-2", Some("fallback"))));
        assert!(auth.is_allowed(&metadata("provider-b", Some("fallback"))));

        // Wrong provider
        assert!(!auth.is_allowed(&metadata("other", Some("claude-sonnet-4-20250514"))));
    }

    #[test]
    fn test_is_allowed_empty_list_allows_any() {
        // Empty models list means "all allowed"
        let auth = Authorization {
            rules: vec![rule(&["provider-a"], &[]), rule(&[], &["gpt-4o"])],
        };
        assert!(auth.is_allowed(&metadata("provider-a", Some("anything"))));
        assert!(auth.is_allowed(&metadata("provider-a", None)));
        assert!(!auth.is_allowed(&metadata("provider-b", Some("claude-sonnet-4-20250514"))));
        assert!(auth.is_allowed(&metadata("provider-b", Some("gpt-4o"))));
    }

    #[test]
    fn test_deny_all() {
        let auth = Authorization::deny_all();
        assert!(!auth.is_allowed(&metadata("anything", None)));
        assert!(!auth.is_allowed(&metadata("anything", Some("model"))));
    }
}
