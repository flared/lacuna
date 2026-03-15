use serde::Deserialize;

use crate::inspector::protocol::ProtocolInspector;
use crate::inspector::protocol::sse::{SseEvent, SseProtocol};
use crate::inspector::protocol::text::{TextBody, TextProtocol};

use super::{ApiTypeHandler, Inspector, MetadataInspector, ResponseMetadata};

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

    fn inspector(&self, _status: u16, headers: &http::HeaderMap) -> MetadataInspector {
        if is_event_stream(headers) {
            Box::new(ProtocolInspector::new(
                SseProtocol::new(),
                AnthropicSseInspector {
                    input_tokens: None,
                    output_tokens: None,
                },
            ))
        } else {
            Box::new(ProtocolInspector::new(
                TextProtocol::new(),
                AnthropicJsonInspector { metadata: None },
            ))
        }
    }
}

fn is_event_stream(headers: &http::HeaderMap) -> bool {
    headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("text/event-stream"))
}

struct AnthropicSseInspector {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

impl Inspector<SseEvent> for AnthropicSseInspector {
    type Output = ResponseMetadata;

    fn feed(&mut self, event: SseEvent) {
        if let Ok(evt) = serde_json::from_str::<SseEventType>(&event.data) {
            match evt.r#type.as_str() {
                "message_start" => {
                    if let Ok(msg) = serde_json::from_str::<MessageStartData>(&event.data) {
                        self.input_tokens = msg.message.usage.input_tokens;
                    }
                }
                "message_delta" => {
                    if let Ok(delta) = serde_json::from_str::<AnthropicDataWithUsage>(&event.data) {
                        self.output_tokens = delta.usage.output_tokens;
                    }
                }
                _ => {}
            }
        }
    }

    fn finish(self: Box<Self>) -> Result<ResponseMetadata, anyhow::Error> {
        if self.input_tokens.is_none() && self.output_tokens.is_none() {
            return Err(anyhow::anyhow!("no token usage found in SSE stream"));
        }
        let response_metadata = ResponseMetadata {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
        };
        Ok(response_metadata)
    }
}

struct AnthropicJsonInspector {
    metadata: Option<Result<ResponseMetadata, anyhow::Error>>,
}

impl Inspector<TextBody> for AnthropicJsonInspector {
    type Output = ResponseMetadata;

    fn feed(&mut self, body: TextBody) {
        self.metadata = Some(parse_anthropic_json(&body.data));
    }

    fn finish(self: Box<Self>) -> Result<ResponseMetadata, anyhow::Error> {
        self.metadata
            .unwrap_or_else(|| Err(anyhow::anyhow!("no response body")))
    }
}

fn parse_anthropic_json(data: &[u8]) -> Result<ResponseMetadata, anyhow::Error> {
    let parsed = serde_json::from_slice::<AnthropicDataWithUsage>(data)?;
    Ok(ResponseMetadata {
        input_tokens: parsed.usage.input_tokens,
        output_tokens: parsed.usage.output_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_json_inspector() -> MetadataInspector {
        AnthropicMessagesHandler.inspector(200, &http::HeaderMap::new())
    }

    fn make_sse_inspector() -> MetadataInspector {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "text/event-stream".parse().unwrap(),
        );
        AnthropicMessagesHandler.inspector(200, &headers)
    }

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
        let mut inspector = make_json_inspector();
        inspector.feed(body);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }

    #[test]
    fn inspect_response_missing_usage() {
        let mut inspector = make_json_inspector();
        inspector.feed(br#"{"id": "msg_123", "type": "message"}"#);
        assert!(inspector.finish().is_err());
    }

    #[test]
    fn inspect_response_invalid_json() {
        let mut inspector = make_json_inspector();
        inspector.feed(b"not json");
        assert!(inspector.finish().is_err());
    }

    #[test]
    fn inspect_response_sse_stream() {
        let body = br#"event: message_start
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
        let mut inspector = make_sse_inspector();
        inspector.feed(body);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }

    #[test]
    fn inspect_sse_chunked() {
        let chunk1 = br#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":25,"output_tokens":1}}}

"#;
        let chunk2 = br#"event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":150}}

"#;
        let mut inspector = make_sse_inspector();
        inspector.feed(chunk1);
        inspector.feed(chunk2);
        let metadata = inspector.finish().unwrap();
        assert_eq!(metadata.input_tokens, Some(25));
        assert_eq!(metadata.output_tokens, Some(150));
    }
}
