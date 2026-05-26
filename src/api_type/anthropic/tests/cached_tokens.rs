use super::*;

#[tokio::test]
async fn inspect_request_uniform_ttl() {
    let body = br#"{
        "model": "claude-sonnet-4-20250514",
        "system": [{"type": "text", "text": "You are helpful.", "cache_control": {"type": "ephemeral", "ttl": 3600}}],
        "messages": [{"role": "user", "content": [{"type": "text", "text": "Hi", "cache_control": {"type": "ephemeral", "ttl": 3600}}]}]
    }"#;
    let request = make_request(body);
    let (result, _) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.cache_ttl_secs, Some(3600));
}

#[tokio::test]
async fn inspect_request_mixed_ttls() {
    let body = br#"{
        "model": "claude-sonnet-4-20250514",
        "system": [{"type": "text", "text": "You are helpful.", "cache_control": {"type": "ephemeral", "ttl": 300}}],
        "messages": [{"role": "user", "content": [{"type": "text", "text": "Hi", "cache_control": {"type": "ephemeral", "ttl": 3600}}]}]
    }"#;
    let request = make_request(body);
    let (result, _) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.cache_ttl_secs, None);
}

#[tokio::test]
async fn inspect_request_default_ttl() {
    let body = br#"{
        "model": "claude-sonnet-4-20250514",
        "system": [{"type": "text", "text": "You are helpful.", "cache_control": {"type": "ephemeral"}}]
    }"#;
    let request = make_request(body);
    let (result, _) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.cache_ttl_secs, Some(300));
}

#[tokio::test]
async fn inspect_request_no_cache_control() {
    let body = br#"{
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Hi"}]
    }"#;
    let request = make_request(body);
    let (result, _) = AnthropicMessagesHandler.inspect_request(request).await;
    let metadata = result.unwrap();
    assert_eq!(metadata.cache_ttl_secs, None);
}

#[test]
fn inspect_json_cache_creation_breakdown() {
    let body = br#"{
        "id": "msg_123",
        "type": "message",
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 348,
            "cache_read_input_tokens": 1800,
            "cache_creation": {
                "ephemeral_5m_input_tokens": 248,
                "ephemeral_1h_input_tokens": 100
            }
        }
    }"#;
    let mut inspector = make_json_inspector();
    inspector.feed(body);
    let metadata = inspector.finish().unwrap();
    let map = metadata.cache_creation_tokens.unwrap();
    assert_eq!(map.get("5m"), Some(&248));
    assert_eq!(map.get("1h"), Some(&100));
    assert_eq!(metadata.cache_read_input_tokens, Some(1800));
}

#[test]
fn inspect_json_no_cache_fields() {
    let body = br#"{
        "id": "msg_123",
        "type": "message",
        "usage": {"input_tokens": 25, "output_tokens": 150}
    }"#;
    let mut inspector = make_json_inspector();
    inspector.feed(body);
    let metadata = inspector.finish().unwrap();
    assert_eq!(metadata.cache_creation_tokens, None);
    assert_eq!(metadata.cache_read_input_tokens, None);
}

#[test]
fn inspect_sse_cache_with_ttl_hint_5m() {
    let body = br#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":25,"output_tokens":1,"cache_creation_input_tokens":348,"cache_read_input_tokens":0}}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":150}}

"#;
    let mut inspector = make_sse_inspector_with_hint(Some(300));
    inspector.feed(body);
    let metadata = inspector.finish().unwrap();
    let map = metadata.cache_creation_tokens.unwrap();
    assert_eq!(map.get("5m"), Some(&348));
    assert_eq!(metadata.cache_read_input_tokens, Some(0));
}

#[test]
fn inspect_sse_cache_with_ttl_hint_1h() {
    let body = br#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":25,"output_tokens":1,"cache_creation_input_tokens":500,"cache_read_input_tokens":200}}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}

"#;
    let mut inspector = make_sse_inspector_with_hint(Some(3600));
    inspector.feed(body);
    let metadata = inspector.finish().unwrap();
    let map = metadata.cache_creation_tokens.unwrap();
    assert_eq!(map.get("1h"), Some(&500));
    assert_eq!(metadata.cache_read_input_tokens, Some(200));
}

#[test]
fn inspect_sse_cache_no_ttl_hint() {
    let body = br#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":25,"output_tokens":1,"cache_creation_input_tokens":348,"cache_read_input_tokens":0}}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":150}}

"#;
    let mut inspector = make_sse_inspector_with_hint(None);
    inspector.feed(body);
    let metadata = inspector.finish().unwrap();
    let map = metadata.cache_creation_tokens.unwrap();
    assert_eq!(map.get("unknown"), Some(&348));
}
