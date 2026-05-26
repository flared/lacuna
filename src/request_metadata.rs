use crate::http_middleware::auth::Identity;
use crate::user_agent::UserAgentMetadata;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseMetadata {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_tokens: Option<HashMap<String, u64>>,
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestInspectionMetadata {
    pub model: Option<String>,
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug)]
pub struct RequestMetadata {
    pub provider_key: String,
    pub api_handler_id: String,
    pub user_identity: Option<Identity>,
    pub user_agent: Option<UserAgentMetadata>,
    pub inspected: RequestInspectionMetadata,
    pub labels: HashMap<String, String>,
}

impl RequestMetadata {
    pub fn labels(&self) -> Vec<(String, String)> {
        let user = match &self.user_identity {
            Some(Identity::LoginUser(email)) => email.clone(),
            _ => String::new(),
        };
        let model = self.inspected.model.clone().unwrap_or_default();
        let user_agent = self
            .user_agent
            .as_ref()
            .map(|ua| ua.normalized.clone())
            .unwrap_or_default();

        let mut out = vec![
            ("provider".to_owned(), self.provider_key.clone()),
            ("handler".to_owned(), self.api_handler_id.clone()),
            ("user".to_owned(), user),
            ("model".to_owned(), model),
            ("user_agent".to_owned(), user_agent),
        ];
        for (k, v) in &self.labels {
            out.push((format!("label_{k}"), v.clone()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metadata(labels: HashMap<String, String>) -> RequestMetadata {
        RequestMetadata {
            provider_key: "myprovider".to_owned(),
            api_handler_id: "chat".to_owned(),
            user_identity: None,
            user_agent: None,
            inspected: RequestInspectionMetadata {
                model: Some("gpt-4o".to_owned()),
                ..Default::default()
            },
            labels,
        }
    }

    #[test]
    fn labels_includes_static_fields_and_dynamic() {
        let custom = HashMap::from([
            ("env".to_owned(), "production".to_owned()),
            ("team".to_owned(), "platform".to_owned()),
        ]);
        let metadata = make_metadata(custom);
        let labels = metadata.labels();
        assert!(
            labels
                .iter()
                .any(|(k, v)| k == "provider" && v == "myprovider")
        );
        assert!(labels.iter().any(|(k, v)| k == "handler" && v == "chat"));
        assert!(labels.iter().any(|(k, v)| k == "model" && v == "gpt-4o"));
        assert!(
            labels
                .iter()
                .any(|(k, v)| k == "label_env" && v == "production")
        );
        assert!(
            labels
                .iter()
                .any(|(k, v)| k == "label_team" && v == "platform")
        );
    }
}
