use bytes::BytesMut;

use super::Protocol;

/// An SSE event data payload.
#[derive(Debug)]
pub struct SseEvent {
    pub data: String,
}

/// Sans-IO SSE protocol parser.
/// Parses `data: ...` lines from an SSE byte stream and emits `SseEvent` frames.
#[derive(Debug, Default)]
pub struct SseProtocol {
    line_buf: BytesMut,
}

impl SseProtocol {
    pub fn new() -> Self {
        Self {
            line_buf: BytesMut::new(),
        }
    }
}

impl Protocol for SseProtocol {
    type Output = SseEvent;

    fn feed(&mut self, chunk: &[u8], on_output: &mut dyn FnMut(SseEvent)) {
        self.line_buf.extend_from_slice(chunk);

        loop {
            let Some(newline_pos) = self.line_buf.iter().position(|&b| b == b'\n') else {
                break;
            };

            let line_bytes = self.line_buf.split_to(newline_pos + 1);
            let line = &line_bytes[..line_bytes.len() - 1]; // strip \n
            let line = if line.last() == Some(&b'\r') {
                &line[..line.len() - 1]
            } else {
                line
            };

            if let Ok(s) = std::str::from_utf8(line)
                && let Some(data) = s.strip_prefix("data: ")
            {
                on_output(SseEvent {
                    data: data.to_owned(),
                });
            }
        }
    }

    fn finish(&mut self, on_output: &mut dyn FnMut(SseEvent)) {
        if self.line_buf.is_empty() {
            return;
        }
        if let Ok(s) = std::str::from_utf8(&self.line_buf)
            && let Some(data) = s.strip_prefix("data: ")
        {
            on_output(SseEvent {
                data: data.to_owned(),
            });
        }
        self.line_buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_events(protocol: &mut SseProtocol, chunks: &[&[u8]]) -> Vec<String> {
        let mut events = Vec::new();
        for chunk in chunks {
            protocol.feed(chunk, &mut |event: SseEvent| {
                events.push(event.data);
            });
        }
        protocol.finish(&mut |event: SseEvent| {
            events.push(event.data);
        });
        events
    }

    #[test]
    fn single_event() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"data: hello\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn multiple_events() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(
            &mut protocol,
            &[b"event: message\ndata: first\n\nevent: message\ndata: second\n\n"],
        );
        assert_eq!(events, vec!["first", "second"]);
    }

    #[test]
    fn split_across_chunks() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"data: hel", b"lo\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn split_at_newline_boundary() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"data: hello", b"\ndata: world\n"]);
        assert_eq!(events, vec!["hello", "world"]);
    }

    #[test]
    fn ignores_non_data_lines() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(
            &mut protocol,
            &[b"event: message\nid: 123\ndata: payload\nretry: 1000\n\n"],
        );
        assert_eq!(events, vec!["payload"]);
    }

    #[test]
    fn handles_crlf() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"data: hello\r\n"]);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn json_data() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(
            &mut protocol,
            &[b"data: {\"type\":\"message_start\",\"tokens\":25}\n"],
        );
        assert_eq!(events, vec!["{\"type\":\"message_start\",\"tokens\":25}"]);
    }

    #[test]
    fn byte_at_a_time() {
        let mut protocol = SseProtocol::new();
        let input = b"data: hello\n";
        let chunks: Vec<&[u8]> = input.iter().map(std::slice::from_ref).collect();
        let events = collect_events(&mut protocol, &chunks);
        assert_eq!(events, vec!["hello"]);
    }

    #[test]
    fn unterminated_line_emitted_on_finish() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[b"data: trailing"]);
        assert_eq!(events, vec!["trailing"]);
    }

    #[test]
    fn empty_input() {
        let mut protocol = SseProtocol::new();
        let events = collect_events(&mut protocol, &[]);
        assert!(events.is_empty());
    }
}
