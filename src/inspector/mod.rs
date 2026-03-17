pub mod callback;
mod content_decoder;
pub mod decoding_inspector;
pub mod protocol;
pub mod static_inspector;
pub mod stream;

pub use callback::CallbackInspector;
pub use decoding_inspector::DecodingInspector;
pub use static_inspector::StaticInspector;

/// Incremental inspector that accumulates output from fed frames.
pub trait Inspector<F>: Send {
    type Output;

    /// Feed a frame of data.
    fn feed(&mut self, frame: F);

    /// Signal end-of-stream. Returns accumulated output.
    fn finish(self: Box<Self>) -> Result<Self::Output, anyhow::Error>;
}

/// Boxed byte-level inspector.
pub type ByteInspector<T> = Box<dyn for<'a> Inspector<&'a [u8], Output = T>>;
