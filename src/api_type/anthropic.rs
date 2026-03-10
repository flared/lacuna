use super::ResponseMetadata;
use serde::Deserialize;

use super::ApiTypeHandler;

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    usage: AnthropicUsage,
}

pub struct AnthropicMessagesHandler;

impl ApiTypeHandler for AnthropicMessagesHandler {
    fn id(&self) -> &'static str {
        "anthropic_messages"
    }

    fn inspect_response(
        &self,
        response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        let parsed = serde_json::from_slice::<AnthropicResponse>(response.body())?;
        let metadata = ResponseMetadata {
            input_tokens: Some(parsed.usage.input_tokens),
            output_tokens: Some(parsed.usage.output_tokens),
        };
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::response_with_body;

    #[test]
    fn inspect_response_full() {
        let body = br#"{
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hi!"}],
            "model": "claude-sonnet-4-20250514",
            "usage": {"input_tokens": 25, "output_tokens": 150}
        }"#;
        let metadata = AnthropicMessagesHandler
            .inspect_response(&response_with_body(body))
            .unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }

    #[test]
    fn inspect_response_missing_usage() {
        let response = response_with_body(br#"{"id": "msg_123", "type": "message"}"#);
        assert!(
            AnthropicMessagesHandler
                .inspect_response(&response)
                .is_err()
        );
    }

    #[test]
    fn inspect_response_invalid_json() {
        assert!(
            AnthropicMessagesHandler
                .inspect_response(&response_with_body(b"not json"))
                .is_err()
        );
    }
}
