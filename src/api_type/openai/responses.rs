use super::super::{ApiTypeHandler, ResponseMetadata};
use serde::Deserialize;

// https://platform.openai.com/docs/api-reference/responses/object
#[derive(Debug, Deserialize)]
struct OpenAiResponsesUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesResponse {
    usage: OpenAiResponsesUsage,
}

pub struct OpenAiResponsesHandler;

impl ApiTypeHandler for OpenAiResponsesHandler {
    fn id(&self) -> &'static str {
        "openai_responses"
    }

    fn inspect_response(
        &self,
        response: &http::Response<bytes::Bytes>,
    ) -> Result<ResponseMetadata, anyhow::Error> {
        let parsed = serde_json::from_slice::<OpenAiResponsesResponse>(response.body())?;
        Ok(ResponseMetadata {
            input_tokens: Some(parsed.usage.input_tokens),
            output_tokens: Some(parsed.usage.output_tokens),
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
            "id": "resp_abc123",
            "object": "response",
            "model": "gpt-4o",
            "usage": {"input_tokens": 10, "output_tokens": 30, "total_tokens": 40}
        }"#;
        let metadata = OpenAiResponsesHandler
            .inspect_response(&response_with_body(body))
            .unwrap();
        assert_eq!(metadata.input_tokens, Some(10));
        assert_eq!(metadata.output_tokens, Some(30));
    }

    #[test]
    fn inspect_response_missing_usage() {
        let response = response_with_body(br#"{"id": "resp_abc123", "object": "response"}"#);
        assert!(OpenAiResponsesHandler.inspect_response(&response).is_err());
    }

    #[test]
    fn inspect_response_invalid_json() {
        assert!(
            OpenAiResponsesHandler
                .inspect_response(&response_with_body(b"not json"))
                .is_err()
        );
    }
}
