mod anthropic;
mod bedrock;
mod gemini;
mod google;
mod openai;

use async_trait::async_trait;
use regex::Regex;
use std::sync::LazyLock;

pub use crate::inspector::{ByteInspector, Inspector, StaticInspector};
pub use crate::request_metadata::{RequestInspectionMetadata, ResponseMetadata};

pub type ResponseMetadataInspector = ByteInspector<ResponseMetadata>;

#[derive(Debug, Clone, PartialEq)]
pub enum ApiType {
    OpenAiChatCompletion,
    OpenAiResponses,
    AnthropicMessages,
    GeminiGenerateContent,
    BedrockModelInvoke,
    GoogleGenerateContent,
    GoogleRawPredict,
}

#[async_trait]
pub trait ApiTypeHandler {
    fn id(&self) -> &'static str;

    /// Create an inspector for this response.
    /// The inspector pairs a protocol parser with provider-specific metadata extraction.
    /// Defaults to a no-op inspector that returns empty metadata.
    fn response_inspector(
        &self,
        _status: u16,
        _headers: &http::HeaderMap,
        _request_metadata: &RequestInspectionMetadata,
    ) -> ResponseMetadataInspector {
        Box::new(StaticInspector::default())
    }

    /// Inspect the request and extract metadata.
    /// Reads the request body upfront if needed, then returns the metadata and a reconstructed request.
    /// Defaults to a no-op that returns empty metadata and the request unchanged.
    async fn inspect_request(
        &self,
        request: axum::extract::Request,
    ) -> (
        anyhow::Result<RequestInspectionMetadata>,
        axum::extract::Request,
    ) {
        (Ok(RequestInspectionMetadata::default()), request)
    }

    /// Apply `rewrite` to the request wherever this api type carries the model name (url or request body).
    fn rewrite_model_in_request(
        &self,
        _request: axum::extract::Request,
        _new_name: &str,
    ) -> anyhow::Result<axum::extract::Request> {
        anyhow::bail!(
            "model rewrite is not supported for api type '{}'",
            self.id()
        )
    }
}

impl ApiType {
    pub fn handler(&self) -> Box<dyn ApiTypeHandler + Send + Sync> {
        match self {
            ApiType::OpenAiChatCompletion => Box::new(openai::OpenAiChatCompletionHandler),
            ApiType::OpenAiResponses => Box::new(openai::OpenAiResponsesHandler),
            ApiType::AnthropicMessages => Box::new(anthropic::AnthropicMessagesHandler),
            ApiType::GeminiGenerateContent => Box::new(gemini::GeminiGenerateContentHandler),
            ApiType::BedrockModelInvoke => Box::new(bedrock::BedrockModelInvokeHandler),
            ApiType::GoogleGenerateContent => Box::new(google::GoogleGenerateContentHandler),
            ApiType::GoogleRawPredict => Box::new(google::GoogleRawPredictHandler),
        }
    }
}

static RE_OPENAI_CHAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/chat/completions").unwrap());
static RE_OPENAI_RESPONSES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/responses").unwrap());
static RE_ANTHROPIC_MESSAGES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/messages").unwrap());
static RE_BEDROCK_MODEL_INVOKE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/model/.+/invoke").unwrap());
static RE_GOOGLE_GENERATE_CONTENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/v1/projects/.+:generateContent$").unwrap());
static RE_GOOGLE_RAW_PREDICT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":rawPredict$").unwrap());
static RE_GEMINI_GENERATE_CONTENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":generateContent$").unwrap());

pub fn api_type_for_path(path: &str) -> Option<ApiType> {
    if RE_OPENAI_CHAT.is_match(path) {
        Some(ApiType::OpenAiChatCompletion)
    } else if RE_OPENAI_RESPONSES.is_match(path) {
        Some(ApiType::OpenAiResponses)
    } else if RE_ANTHROPIC_MESSAGES.is_match(path) {
        Some(ApiType::AnthropicMessages)
    } else if RE_BEDROCK_MODEL_INVOKE.is_match(path) {
        Some(ApiType::BedrockModelInvoke)
    } else if RE_GOOGLE_RAW_PREDICT.is_match(path) {
        Some(ApiType::GoogleRawPredict)
    } else if RE_GOOGLE_GENERATE_CONTENT.is_match(path) {
        Some(ApiType::GoogleGenerateContent)
    } else if RE_GEMINI_GENERATE_CONTENT.is_match(path) {
        Some(ApiType::GeminiGenerateContent)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn api_type_for_path_cases() {
        let cases: Vec<(&str, Option<ApiType>)> = vec![
            // OpenAI Chat
            ("/v1/chat/completions", Some(ApiType::OpenAiChatCompletion)),
            (
                "/v1/chat/completions?stream=true",
                Some(ApiType::OpenAiChatCompletion),
            ),
            // OpenAI Responses
            ("/v1/responses", Some(ApiType::OpenAiResponses)),
            ("/v1/responses/resp_123", Some(ApiType::OpenAiResponses)),
            // Anthropic Messages
            ("/v1/messages", Some(ApiType::AnthropicMessages)),
            ("/v1/messages?stream=true", Some(ApiType::AnthropicMessages)),
            // Gemini Generate Content
            (
                "/v1/models/gemini-2.0-flash:generateContent",
                Some(ApiType::GeminiGenerateContent),
            ),
            // Bedrock Model Invoke
            (
                "/model/us.anthropic.claude-sonnet-4-5/invoke",
                Some(ApiType::BedrockModelInvoke),
            ),
            // Google Generate Content
            (
                "/v1/projects/my-proj/locations/us/publishers/google/models/gemini-2.5-pro:generateContent",
                Some(ApiType::GoogleGenerateContent),
            ),
            // Google Raw Predict
            (
                "/v1/projects/my-proj/locations/us/publishers/google/models/claude-opus-4-5:rawPredict",
                Some(ApiType::GoogleRawPredict),
            ),
            // Unrelated paths
            ("/health", None),
            ("/v2/something", None),
            ("/anything", None),
        ];
        for (path, expected) in cases {
            assert_eq!(api_type_for_path(path), expected, "path: {path}");
        }
    }

    #[test]
    fn handler_ids() {
        // All handlers should have a unique id.
        let cases: Vec<(ApiType, &str)> = vec![
            (ApiType::OpenAiChatCompletion, "openai_chat_completion"),
            (ApiType::OpenAiResponses, "openai_responses"),
            (ApiType::AnthropicMessages, "anthropic_messages"),
            (ApiType::GeminiGenerateContent, "gemini_generate_content"),
            (ApiType::BedrockModelInvoke, "bedrock_model_invoke"),
            (ApiType::GoogleGenerateContent, "google_generate_content"),
            (ApiType::GoogleRawPredict, "google_raw_predict"),
        ];
        for (api_type, expected_name) in cases {
            assert_eq!(api_type.handler().id(), expected_name, "{api_type:?}");
        }
    }

    #[test]
    fn rewrite_model_in_request_errors_for_unsupported_api_types() {
        let unsupported = [
            ApiType::OpenAiChatCompletion,
            ApiType::OpenAiResponses,
            ApiType::AnthropicMessages,
            ApiType::GeminiGenerateContent,
            ApiType::GoogleGenerateContent,
            ApiType::GoogleRawPredict,
        ];
        for api_type in unsupported {
            let handler = api_type.handler();
            let request = axum::http::Request::builder()
                .uri("/v1/messages")
                .body(Body::empty())
                .unwrap();
            let err = handler
                .rewrite_model_in_request(request, "target")
                .expect_err(&format!("{api_type:?} should not support model rewrite"));

            assert!(
                err.to_string().contains(handler.id()),
                "{api_type:?}: error should mention the api type id, got: {err}"
            );
        }
    }
}
