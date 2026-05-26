use axum::body::Body;

use super::*;
use crate::api_type::ApiTypeHandler;
use crate::request_metadata::RequestInspectionMetadata;

mod cached_tokens;

pub(super) fn make_request(body: &'static [u8]) -> axum::extract::Request {
    axum::http::Request::builder()
        .uri("http://localhost/v1/messages")
        .body(Body::from(body))
        .unwrap()
}

pub(super) fn make_json_inspector() -> ResponseMetadataInspector {
    AnthropicMessagesHandler.response_inspector(
        200,
        &http::HeaderMap::new(),
        &RequestInspectionMetadata::default(),
    )
}

pub(super) fn make_sse_inspector() -> ResponseMetadataInspector {
    make_sse_inspector_with_hint(None)
}

pub(super) fn make_sse_inspector_with_hint(
    cache_ttl_secs: Option<u64>,
) -> ResponseMetadataInspector {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        "text/event-stream".parse().unwrap(),
    );
    AnthropicMessagesHandler.response_inspector(
        200,
        &headers,
        &RequestInspectionMetadata {
            cache_ttl_secs,
            ..Default::default()
        },
    )
}

#[tokio::test]
async fn inspect_request_model() {
    let request = make_request(br#"{"model": "claude-sonnet-4-20250514", "max_tokens": 1024, "messages": [{"role": "user", "content": "Hi"}]}"#);
    let (result, _request) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.model, Some("claude-sonnet-4-20250514".to_owned()));
}

#[tokio::test]
async fn inspect_request_no_model() {
    let request = make_request(br#"{"max_tokens": 1024, "messages": []}"#);
    let (result, _request) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.model, None);
}

#[tokio::test]
async fn inspect_request_invalid_json() {
    let request = make_request(b"not json");
    let (result, _request) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.model, None);
}

#[tokio::test]
async fn inspect_request_empty_body() {
    let request = make_request(b"");
    let (result, _request) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.model, None);
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
