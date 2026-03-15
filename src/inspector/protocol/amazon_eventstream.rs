use aws_smithy_eventstream::frame::{DecodedFrame, MessageFrameDecoder};
use bytes::BytesMut;

use super::Protocol;

/// A decoded Amazon EventStream event payload.
#[derive(Debug)]
pub struct EventstreamEvent {
    pub payload: bytes::Bytes,
}

/// Sans-IO Amazon EventStream binary protocol parser.
/// Decodes binary eventstream frames and emits `EventstreamEvent` for each complete message.
#[derive(Debug, Default)]
pub struct AmazonEventstreamProtocol {
    decoder: MessageFrameDecoder,
    buf: BytesMut,
}

impl Protocol for AmazonEventstreamProtocol {
    type Output = EventstreamEvent;

    fn feed(&mut self, chunk: &[u8], on_output: &mut dyn FnMut(EventstreamEvent)) {
        self.buf.extend_from_slice(chunk);
        loop {
            match self.decoder.decode_frame(&mut self.buf) {
                Ok(DecodedFrame::Complete(message)) => {
                    on_output(EventstreamEvent {
                        payload: message.payload().to_owned(),
                    });
                }
                Ok(DecodedFrame::Incomplete) => break,
                Err(_) => break,
            }
        }
    }

    fn finish(&mut self, _on_output: &mut dyn FnMut(EventstreamEvent)) {
        // All complete frames are emitted during feed().
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use aws_smithy_types::event_stream::{Header, HeaderValue, Message};

    /// Build a raw Amazon EventStream binary frame with the given payload.
    pub(crate) fn build_eventstream_frame(payload: &[u8]) -> Vec<u8> {
        let message = Message::new_from_parts(
            vec![
                Header::new(":event-type", HeaderValue::String("chunk".into())),
                Header::new(
                    ":content-type",
                    HeaderValue::String("application/json".into()),
                ),
                Header::new(":message-type", HeaderValue::String("event".into())),
            ],
            payload.to_vec(),
        );
        let mut buf = Vec::new();
        aws_smithy_eventstream::frame::write_message_to(&message, &mut buf).unwrap();
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::build_eventstream_frame;
    use super::*;

    #[test]
    fn decode_single_frame() {
        let payload = b"hello";
        let frame = build_eventstream_frame(payload);

        let mut protocol = AmazonEventstreamProtocol::default();
        let mut events = Vec::new();
        protocol.feed(&frame, &mut |e| events.push(e));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload.as_ref(), b"hello");
    }

    #[test]
    fn decode_multiple_frames() {
        let frame1 = build_eventstream_frame(b"first");
        let frame2 = build_eventstream_frame(b"second");
        let mut combined = frame1;
        combined.extend_from_slice(&frame2);

        let mut protocol = AmazonEventstreamProtocol::default();
        let mut events = Vec::new();
        protocol.feed(&combined, &mut |e| events.push(e));

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].payload.as_ref(), b"first");
        assert_eq!(events[1].payload.as_ref(), b"second");
    }

    #[test]
    fn decode_chunked_frame() {
        let frame = build_eventstream_frame(b"chunked");
        let mid = frame.len() / 2;

        let mut protocol = AmazonEventstreamProtocol::default();
        let mut events = Vec::new();
        protocol.feed(&frame[..mid], &mut |e| events.push(e));
        assert!(events.is_empty());
        protocol.feed(&frame[mid..], &mut |e| events.push(e));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload.as_ref(), b"chunked");
    }
}
