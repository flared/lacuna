use bytes::BytesMut;

use super::Protocol;

/// A spec-compliant SSE event.
/// See: https://html.spec.whatwg.org/multipage/server-sent-events.html
#[derive(Debug)]
pub struct SseEvent {
    /// Concatenated data lines (joined with '\n').
    pub data: String,
    /// Event type, defaults to "message".
    pub event_type: String,
    /// Last event ID, persists across events. Empty if unset.
    pub last_event_id: String,
    /// Reconnection time in milliseconds, only if retry field was present.
    pub retry: Option<u64>,
}

/// State for parsing the event stream after the BOM check is done.
#[derive(Debug)]
pub struct EventStreamParsingState {
    line_buf: BytesMut,
    data_buf: String,
    event_type_buf: String,
    last_event_id_buf: String,
    retry_buf: Option<u64>,
    has_data: bool,
}

/// Sans-IO spec-compliant SSE protocol parser.
/// Parses an SSE byte stream and emits `SseEvent` frames per the SSE specification.
///
/// Starts in `BomParsing` to check for a leading UTF-8 BOM, then permanently
/// transitions to `EventStreamParsing` for the rest of the stream.
#[derive(Debug)]
pub enum SseProtocol {
    /// Initial state: buffering bytes until BOM decision can be made.
    BomParsing(BytesMut),
    /// Normal parsing state — BOM already handled.
    EventStreamParsing(EventStreamParsingState),
}

impl Default for SseProtocol {
    fn default() -> Self {
        Self::new()
    }
}


impl Protocol for SseProtocol {
    type Output = SseEvent;

    fn feed(&mut self, chunk: &[u8], on_output: &mut dyn FnMut(SseEvent)) {
        match self {
            Self::BomParsing(buf) => {
                buf.extend_from_slice(chunk);
                *self = Self::parse_bom(std::mem::take(buf), false);
            }
            Self::EventStreamParsing(state) => {
                state.line_buf.extend_from_slice(chunk);
            }
        }

        if let Self::EventStreamParsing(state) = self {
            state.parse_events(false, on_output);
        }
    }

    fn finish(&mut self, on_output: &mut dyn FnMut(SseEvent)) {
        if let Self::BomParsing(buf) = self {
            *self = Self::parse_bom(std::mem::take(buf), true);
        }

        let Self::EventStreamParsing(state) = self else {
            unreachable!();
        };
        state.parse_events(true, on_output);
        state.clear();
    }
}


impl SseProtocol {
    pub fn new() -> Self {
        Self::BomParsing(BytesMut::new())
    }

    /// Consume the BOM buffer and return the next state.
    /// Returns `BomParsing` if more data is needed, `EventStreamParsing` otherwise.
    fn parse_bom(mut buf: BytesMut, at_eof: bool) -> Self {
        if !buf.is_empty() && buf[0] == 0xEF {
            if !at_eof && buf.len() < 3 {
                return Self::BomParsing(buf); // need more data
            }
            if buf[1] == 0xBB && buf[2] == 0xBF {
                let _ = buf.split_to(3);
            }
        }
        Self::EventStreamParsing(EventStreamParsingState::new(buf))
    }
}

impl EventStreamParsingState {
    fn new(line_buf: BytesMut) -> Self {
        Self {
            line_buf,
            data_buf: String::new(),
            event_type_buf: String::new(),
            last_event_id_buf: String::new(),
            retry_buf: None,
            has_data: false,
        }
    }


    fn parse_events(&mut self, at_eof: bool, on_output: &mut dyn FnMut(SseEvent)) {
        loop {
            let Some((line_end, consume_to)) = Self::find_line_end(&self.line_buf, at_eof) else {
                break;
            };

            let line_bytes = self.line_buf.split_to(consume_to);
            let line = &line_bytes[..line_end];

            if let Ok(s) = std::str::from_utf8(line) {
                self.process_line(s, on_output);
            }
        }
    }

    /// Find the end of a line in the buffer.
    /// Returns `(line_end, consume_to)` where line_end is the index of the line terminator
    /// and consume_to is the index past the full line ending sequence.
    /// `at_eof` controls whether a trailing `\r` is treated as a line ending.
    fn find_line_end(buf: &[u8], at_eof: bool) -> Option<(usize, usize)> {
        for i in 0..buf.len() {
            match buf[i] {
                b'\n' => return Some((i, i + 1)),
                b'\r' => {
                    if i + 1 < buf.len() {
                        if buf[i + 1] == b'\n' {
                            return Some((i, i + 2));
                        } else {
                            return Some((i, i + 1));
                        }
                    } else if at_eof {
                        return Some((i, i + 1));
                    } else {
                        return None; // need more data to know if \r\n
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn process_line(&mut self, line: &str, on_output: &mut dyn FnMut(SseEvent)) {
        if line.is_empty() {
            self.dispatch_event(on_output);
            return;
        }

        if line.starts_with(':') {
            return;
        }

        let (field, value) = if let Some(colon_pos) = line.find(':') {
            let field = &line[..colon_pos];
            let rest = &line[colon_pos + 1..];
            let value = rest.strip_prefix(' ').unwrap_or(rest);
            (field, value)
        } else {
            (line, "")
        };

        match field {
            "data" => {
                if self.has_data {
                    self.data_buf.push('\n');
                }
                self.data_buf.push_str(value);
                self.has_data = true;
            }
            "event" => {
                self.event_type_buf = value.to_owned();
            }
            "id" => {
                if !value.contains('\0') {
                    self.last_event_id_buf = value.to_owned();
                }
            }
            "retry" => {
                if !value.is_empty()
                    && value.bytes().all(|b| b.is_ascii_digit())
                    && let Ok(v) = value.parse::<u64>()
                {
                    self.retry_buf = Some(v);
                }
            }
            _ => {}
        }
    }

    fn dispatch_event(&mut self, on_output: &mut dyn FnMut(SseEvent)) {
        if !self.has_data {
            self.event_type_buf.clear();
            self.retry_buf = None;
            return;
        }

        let event_type = if self.event_type_buf.is_empty() {
            "message".to_owned()
        } else {
            std::mem::take(&mut self.event_type_buf)
        };

        let event = SseEvent {
            data: std::mem::take(&mut self.data_buf),
            event_type,
            last_event_id: self.last_event_id_buf.clone(),
            retry: self.retry_buf.take(),
        };

        self.has_data = false;

        on_output(event);
    }

    fn clear(&mut self) {
        self.line_buf.clear();
        self.data_buf.clear();
        self.event_type_buf.clear();
        self.retry_buf = None;
        self.has_data = false;
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
