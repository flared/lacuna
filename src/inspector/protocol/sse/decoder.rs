use bytes::BytesMut;

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

/// Result of attempting to decode an SSE event from the buffer.
#[derive(Debug)]
pub enum DecodedSseEvent {
    /// A complete event was decoded.
    Complete(SseEvent),
    /// Not enough data to decode a complete event.
    Incomplete,
}

/// Sans-IO SSE decoder. Extracts SSE events from a byte buffer.
#[derive(Debug)]
pub struct SseDecoder {
    read_bom: bool,
    data_buf: String,
    event_type_buf: Option<String>,
    last_event_id_buf: String,
    retry_buf: Option<u64>,
    has_data: bool,
}

impl Default for SseDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SseDecoder {
    pub fn new() -> Self {
        Self {
            read_bom: false,
            data_buf: String::new(),
            event_type_buf: None,
            last_event_id_buf: String::new(),
            retry_buf: None,
            has_data: false,
        }
    }

    /// Try to decode one SSE event from `buf`, consuming bytes as needed.
    /// Returns `Complete(event)` when a blank line triggers dispatch,
    /// or `Incomplete` when more data is needed.
    pub fn decode_event(&mut self, buf: &mut BytesMut) -> DecodedSseEvent {
        match self.decode(buf, false) {
            Some(event) => DecodedSseEvent::Complete(event),
            None => DecodedSseEvent::Incomplete,
        }
    }

    /// Like `decode_event` but with EOF semantics: a trailing `\r` counts as a line ending,
    /// and the final line (if any) is processed even without a terminator.
    pub fn eof(&mut self, buf: &mut BytesMut) -> DecodedSseEvent {
        match self.decode(buf, true) {
            Some(event) => DecodedSseEvent::Complete(event),
            None => DecodedSseEvent::Incomplete,
        }
    }

    fn decode(&mut self, buf: &mut BytesMut, at_eof: bool) -> Option<SseEvent> {
        if !self.read_bom {
            self.decode_bom(buf, at_eof)?;
            self.read_bom = true;
        }

        loop {
            let (line_end, consume_to) = Self::find_line_end(buf, at_eof)?;

            let line_bytes = buf.split_to(consume_to);
            let line = &line_bytes[..line_end];

            if let Ok(s) = std::str::from_utf8(line)
                && let Some(event) = self.process_line(s)
            {
                return Some(event);
            }
        }
    }

    fn decode_bom(&mut self, buf: &mut BytesMut, at_eof: bool) -> Option<()> {
        if !buf.is_empty() && buf[0] == 0xEF {
            if !at_eof && buf.len() < 3 {
                return None;
            }
            if buf.len() >= 3 && buf[1] == 0xBB && buf[2] == 0xBF {
                let _ = buf.split_to(3);
            }
        }
        Some(())
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

    /// Process a single line. Returns `Some(event)` when a blank line triggers dispatch.
    fn process_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch_event();
        }

        if line.starts_with(':') {
            return None;
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
                self.event_type_buf = Some(value.to_owned());
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

        None
    }

    fn dispatch_event(&mut self) -> Option<SseEvent> {
        if !self.has_data {
            self.event_type_buf = None;
            self.retry_buf = None;
            return None;
        }

        let event_type = self
            .event_type_buf
            .take()
            .unwrap_or_else(|| "message".to_owned());

        let event = SseEvent {
            data: std::mem::take(&mut self.data_buf),
            event_type,
            last_event_id: self.last_event_id_buf.clone(),
            retry: self.retry_buf.take(),
        };

        self.has_data = false;

        Some(event)
    }
}
