use crate::http_middleware::auth::Identity;

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
    pub inspected: Option<RequestInspectionMetadata>,
}

impl RequestMetadata {
    pub fn labels(&self) -> [(&'static str, String); 4] {
        let user = match &self.user_identity {
            Some(Identity::LoginUser(email)) => email.clone(),
            _ => String::new(),
        };
        let model = self
            .inspected
            .as_ref()
            .and_then(|m| m.model.clone())
            .unwrap_or_default();

        [
            ("provider", self.provider_key.clone()),
            ("handler", self.api_handler_id.clone()),
            ("user", user),
            ("model", model),
        ]
    }
}
