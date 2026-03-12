use serde::Deserialize;

use super::{ApiTypeHandler, ResponseMetadata};

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

// Payload for both non-streaming and SSE.
#[derive(Debug, Deserialize)]
struct AnthropicDataWithUsage {
    usage: Usage,
}

// SSE streaming event payloads.
#[derive(Debug, Deserialize)]
struct SseEventType {
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct MessageStartData {
    message: AnthropicDataWithUsage,
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
        if is_event_stream(response) {
            inspect_sse_response(response.body())
        } else {
            let parsed = serde_json::from_slice::<AnthropicDataWithUsage>(response.body())?;
            Ok(ResponseMetadata {
                input_tokens: parsed.usage.input_tokens,
                output_tokens: parsed.usage.output_tokens,
            })
        }
    }
}

fn is_event_stream(response: &http::Response<bytes::Bytes>) -> bool {
    response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("text/event-stream"))
}

fn inspect_sse_response(body: &bytes::Bytes) -> Result<ResponseMetadata, anyhow::Error> {
    let text = std::str::from_utf8(body)?;
    let mut input_tokens: Option<u64> = None;
    let mut output_tokens: Option<u64> = None;
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ")
            && let Ok(event) = serde_json::from_str::<SseEventType>(data)
        {
            match event.r#type.as_str() {
                "message_start" => {
                    if let Ok(msg) = serde_json::from_str::<MessageStartData>(data) {
                        input_tokens = msg.message.usage.input_tokens;
                    }
                }
                "message_delta" => {
                    if let Ok(delta) = serde_json::from_str::<AnthropicDataWithUsage>(data) {
                        output_tokens = delta.usage.output_tokens;
                    }
                }
                _ => {}
            }
        }
    }

    if input_tokens.is_none() && output_tokens.is_none() {
        anyhow::bail!("no token usage found in SSE stream");
    }

    Ok(ResponseMetadata {
        input_tokens,
        output_tokens,
    })
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

    #[test]
    fn inspect_response_sse_stream() {
        let body = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":25,"output_tokens":1}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi!"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":150}}

event: message_stop
data: {"type":"message_stop"}

"#;
        let response = http::Response::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, "text/event-stream")
            .body(bytes::Bytes::from(body))
            .unwrap();
        let metadata = AnthropicMessagesHandler
            .inspect_response(&response)
            .unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }
}
