mod decoder;

pub use decoder::SseEvent;

use bytes::BytesMut;
use decoder::{DecodedSseEvent, SseDecoder};

use super::Protocol;

/// Sans-IO spec-compliant SSE protocol parser.
/// Parses an SSE byte stream and emits `SseEvent` frames per the SSE specification.
#[derive(Debug, Default)]
pub struct SseProtocol {
    decoder: SseDecoder,
    buf: BytesMut,
}

impl SseProtocol {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Protocol for SseProtocol {
    type Output = SseEvent;

    fn feed(&mut self, chunk: &[u8], on_output: &mut dyn FnMut(SseEvent)) {
        self.buf.extend_from_slice(chunk);
        while let DecodedSseEvent::Complete(event) = self.decoder.decode_event(&mut self.buf) {
            on_output(event);
        }
    }

    fn finish(&mut self, on_output: &mut dyn FnMut(SseEvent)) {
        while let DecodedSseEvent::Complete(event) = self.decoder.eof(&mut self.buf) {
            on_output(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_events(protocol: &mut SseProtocol, chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut events = Vec::new();
        for chunk in chunks {
            protocol.feed(chunk, &mut |event: SseEvent| {
                events.push(event);
            });
        }
        protocol.finish(&mut |event: SseEvent| {
            events.push(event);
        });
        events
    }

    fn collect_data(protocol: &mut SseProtocol, chunks: &[&[u8]]) -> Vec<String> {
        collect_events(protocol, chunks)
            .into_iter()
            .map(|e| e.data)
            .collect()
    }

    #[test]
    fn single_event() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: hello\n\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn multiple_events() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(
            &mut protocol,
            &[b"event: message\ndata: first\n\nevent: message\ndata: second\n\n"],
        );
        assert_eq!(events, vec!["first", "second"]);
    }

    #[test]
    fn split_across_chunks() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: hel", b"lo\n\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn split_at_newline_boundary() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: hello\n\ndata: world\n", b"\n"]);
        assert_eq!(events, vec!["hello", "world"]);
    }

    #[test]
    fn ignores_non_data_lines() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(
            &mut protocol,
            &[b"event: message\nid: 123\ndata: payload\nretry: 1000\n\n"],
        );
        assert_eq!(events, vec!["payload"]);
    }

    #[test]
    fn handles_crlf() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: hello\r\n\r\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn json_data() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(
            &mut protocol,
            &[b"data: {\"type\":\"message_start\",\"tokens\":25}\n\n"],
        );
        assert_eq!(events, vec!["{\"type\":\"message_start\",\"tokens\":25}"]);
    }

    #[test]
    fn byte_at_a_time() {
        let mut protocol = SseProtocol::new();
        let input = b"data: hello\n\n";
        let chunks: Vec<&[u8]> = input.iter().map(std::slice::from_ref).collect();
        let events = collect_data(&mut protocol, &chunks);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn unterminated_event_not_dispatched() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: trailing"]);
        assert!(events.is_empty());
    }

    #[test]
    fn unterminated_event_after_complete_event() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: first\n\ndata: trailing"]);
        assert_eq!(events, vec!["first"]);
    }

    #[test]
    fn empty_input() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[]);
        assert!(events.is_empty());
    }

    #[test]
    fn multi_line_data() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(
            &mut protocol,
            &[b"data: line1\ndata: line2\ndata: line3\n\n"],
        );
        assert_eq!(events, vec!["line1\nline2\nline3"]);
    }

    #[test]
    fn comments_are_ignored() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(
            &mut protocol,
            &[b": this is a comment\ndata: hello\n: another comment\n\n"],
        );
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn event_type_field() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"event: custom\ndata: payload\n\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "custom");
        assert_eq!(events[0].data, "payload");
    }

    #[test]
    fn default_event_type_is_message() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"data: hello\n\n"]);
        assert_eq!(events[0].event_type, "message");
    }

    #[test]
    fn id_field_persists_across_events() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"id: 1\ndata: first\n\ndata: second\n\n"]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].last_event_id, "1");
        assert_eq!(events[1].last_event_id, "1");
    }

    #[test]
    fn id_with_null_rejected() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(
            &mut protocol,
            &[b"id: abc\ndata: first\n\nid: x\0y\ndata: second\n\n"],
        );
        assert_eq!(events[0].last_event_id, "abc");
        assert_eq!(events[1].last_event_id, "abc");
    }

    #[test]
    fn retry_field() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"retry: 3000\ndata: hello\n\n"]);
        assert_eq!(events[0].retry, Some(3000));
    }

    #[test]
    fn retry_non_digit_ignored() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"retry: abc\ndata: hello\n\n"]);
        assert_eq!(events[0].retry, None);
    }

    #[test]
    fn retry_empty_ignored() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"retry:\ndata: hello\n\n"]);
        assert_eq!(events[0].retry, None);
    }

    #[test]
    fn cr_only_line_endings() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: hello\r\rdata: world\r\r"]);
        assert_eq!(events, vec!["hello", "world"]);
    }

    #[test]
    fn bom_stripping() {
        let mut protocol = SseProtocol::new();
        let mut input = vec![0xEF, 0xBB, 0xBF];
        input.extend_from_slice(b"data: hello\n\n");
        let events = collect_data(&mut protocol, &[&input]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn bom_across_chunks() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[&[0xEF, 0xBB], &[0xBF], b"data: hello\n\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn data_no_space_after_colon() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data:hello\n\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn field_no_colon_treated_as_empty_value() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data\n\n"]);
        assert_eq!(events, vec![""]);
    }

    #[test]
    fn event_without_data_not_dispatched() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"event: ping\nid: 1\n\ndata: real\n\n"]);
        assert_eq!(events, vec!["real"]);
    }

    #[test]
    fn event_type_resets_between_events() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(
            &mut protocol,
            &[b"event: custom\ndata: first\n\ndata: second\n\n"],
        );
        assert_eq!(events[0].event_type, "custom");
        assert_eq!(events[1].event_type, "message");
    }

    #[test]
    fn mixed_line_endings() {
        let mut protocol = SseProtocol::new();
        let events = collect_data(&mut protocol, &[b"data: a\rdata: b\r\ndata: c\n\r\n"]);
        assert_eq!(events, vec!["a\nb\nc"]);
    }
}
