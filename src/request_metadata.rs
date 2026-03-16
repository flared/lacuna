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
