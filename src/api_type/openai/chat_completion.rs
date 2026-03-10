use super::super::{ApiTypeHandler, ResponseMetadata};
use serde::Deserialize;

// https://platform.openai.com/docs/api-reference/chat/object
#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionResponse {
    usage: OpenAiChatCompletionUsage,
}

pub struct OpenAiChatCompletionHandler;

impl ApiTypeHandler for OpenAiChatCompletionHandler {
    fn id(&self) -> &'static str {
        "openai_chat_completion"
    }

    fn inspect_response(
        &self,
        response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        let parsed = serde_json::from_slice::<OpenAiChatCompletionResponse>(response.body())?;
        Ok(ResponseMetadata {
            input_tokens: Some(parsed.usage.prompt_tokens),
            output_tokens: Some(parsed.usage.completion_tokens),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::response_with_body;

    #[test]
    fn inspect_response_full() {
        let body = br#"{
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hi!"}}],
            "usage": {"prompt_tokens": 15, "completion_tokens": 42, "total_tokens": 57}
        }"#;
        let metadata = OpenAiChatCompletionHandler
            .inspect_response(&response_with_body(body))
            .unwrap();
        assert_eq!(metadata.input_tokens, Some(15));
        assert_eq!(metadata.output_tokens, Some(42));
    }

    #[test]
    fn inspect_response_missing_usage() {
        let response =
            response_with_body(br#"{"id": "chatcmpl-abc123", "object": "chat.completion"}"#);
        assert!(
            OpenAiChatCompletionHandler
                .inspect_response(&response)
                .is_err()
        );
    }

    #[test]
    fn inspect_response_invalid_json() {
        assert!(
            OpenAiChatCompletionHandler
                .inspect_response(&response_with_body(b"not json"))
                .is_err()
        );
    }
}
