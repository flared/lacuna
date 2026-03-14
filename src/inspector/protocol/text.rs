use bytes::BytesMut;

use super::Protocol;

/// A complete text body.
#[derive(Debug)]
pub struct TextBody {
    pub data: bytes::Bytes,
}

/// Sans-IO text protocol parser.
/// Buffers the entire body and emits a single `TextBody` on finish.
#[derive(Debug, Default)]
pub struct TextProtocol {
    buf: BytesMut,
}

impl TextProtocol {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
        }
    }
}

impl Protocol for TextProtocol {
    type Output = TextBody;

    fn feed(&mut self, chunk: &[u8], _on_output: &mut dyn FnMut(TextBody)) {
        self.buf.extend_from_slice(chunk);
    }

    fn finish(&mut self, on_output: &mut dyn FnMut(TextBody)) {
        if !self.buf.is_empty() {
            on_output(TextBody {
                data: self.buf.split().freeze(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_bodies(protocol: &mut TextProtocol, chunks: &[&[u8]]) -> Vec<bytes::Bytes> {
        let mut bodies = Vec::new();
        for chunk in chunks {
            protocol.feed(chunk, &mut |body: TextBody| {
                bodies.push(body.data);
            });
        }
        protocol.finish(&mut |body: TextBody| {
            bodies.push(body.data);
        });
        bodies
    }

    #[test]
    fn single_chunk() {
        let mut protocol = TextProtocol::new();
        let bodies = collect_bodies(&mut protocol, &[br#"{"key": "value"}"#]);
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].as_ref(), br#"{"key": "value"}"#);
    }

    #[test]
    fn multiple_chunks() {
        let mut protocol = TextProtocol::new();
        let bodies = collect_bodies(&mut protocol, &[br#"{"key":"#, br#" "value"}"#]);
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].as_ref(), br#"{"key": "value"}"#);
    }

    #[test]
    fn empty_input() {
        let mut protocol = TextProtocol::new();
        let bodies = collect_bodies(&mut protocol, &[]);
        assert!(bodies.is_empty());
    }

    #[test]
    fn nothing_emitted_during_feed() {
        let mut protocol = TextProtocol::new();
        let mut count = 0;
        protocol.feed(b"hello", &mut |_: TextBody| {
            count += 1;
        });
        assert_eq!(count, 0);
    }
}
