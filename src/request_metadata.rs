use crate::http_middleware::auth::Identity;
use crate::user_agent::UserAgentMetadata;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseMetadata {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestInspectionMetadata {
    pub model: Option<String>,
}

#[derive(Debug)]
pub struct RequestMetadata {
    pub provider_key: String,
    pub api_handler_id: String,
    pub user_identity: Option<Identity>,
    pub user_agent: Option<UserAgentMetadata>,
    pub inspected: Option<RequestInspectionMetadata>,
}

impl RequestMetadata {
    pub fn labels(&self) -> [(&'static str, String); 5] {
        let user = match &self.user_identity {
            Some(Identity::LoginUser(email)) => email.clone(),
            _ => String::new(),
        };
        let model = self
            .inspected
            .as_ref()
            .and_then(|m| m.model.clone())
            .unwrap_or_default();
        let user_agent = self
            .user_agent
            .as_ref()
            .map(|ua| ua.normalized.clone())
            .unwrap_or_default();

        [
            ("provider", self.provider_key.clone()),
            ("handler", self.api_handler_id.clone()),
            ("user", user),
            ("model", model),
            ("user_agent", user_agent),
        ]
    }
}
