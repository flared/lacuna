pub mod sse;
pub mod text;

use crate::inspector::Inspector;

/// Sans-IO protocol parser. Generic over its output frame type.
/// Parses a byte stream format and emits parsed frames via callback.
pub trait Protocol: Send {
    type Output;

    /// Feed raw bytes into the parser.
    /// Calls `on_output` for each complete frame parsed.
    fn feed(&mut self, chunk: &[u8], on_output: &mut dyn FnMut(Self::Output));

    /// Signal end-of-stream. May emit final frames.
    fn finish(&mut self, on_output: &mut dyn FnMut(Self::Output));
}

/// Combines a protocol parser with a typed inspector.
/// The protocol handles byte→frame translation; the inspector handles frames.
#[derive(Debug)]
pub struct ProtocolInspector<P, I> {
    protocol: P,
    inspector: I,
}

impl<P, I> ProtocolInspector<P, I> {
    pub fn new(protocol: P, inspector: I) -> Self {
        Self {
            protocol,
            inspector,
        }
    }
}

impl<P, I> Inspector<&[u8]> for ProtocolInspector<P, I>
where
    P: Protocol,
    I: Inspector<P::Output>,
{
    type Output = I::Output;

    fn feed(&mut self, chunk: &[u8]) {
        let inspector = &mut self.inspector;
        self.protocol
            .feed(chunk, &mut |frame| inspector.feed(frame));
    }

    fn finish(self: Box<Self>) -> Result<I::Output, anyhow::Error> {
        let mut this = *self;
        let inspector = &mut this.inspector;
        this.protocol.finish(&mut |frame| inspector.feed(frame));
        Box::new(this.inspector).finish()
    }
}
