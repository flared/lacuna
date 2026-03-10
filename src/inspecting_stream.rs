use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream wrapper that collects chunks as they pass through and fires
/// a callback with the complete body when the stream ends.
pub struct InspectingStream<S> {
    inner_stream: Pin<Box<S>>,
    on_complete: Option<Box<dyn FnOnce(Bytes) + Send>>,
    collected: BytesMut,
}

impl<S: std::fmt::Debug> std::fmt::Debug for InspectingStream<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectingStream")
            .field("inner_stream", &self.inner_stream)
            .field("collected", &self.collected)
            .finish_non_exhaustive()
    }
}

impl<S> InspectingStream<S> {
    pub fn new(stream: S, on_complete: impl FnOnce(Bytes) + Send + 'static) -> Self {
        Self {
            inner_stream: Box::pin(stream),
            on_complete: Some(Box::new(on_complete)),
            collected: BytesMut::new(),
        }
    }
}

impl<S, E> Stream for InspectingStream<S>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    type Item = Result<Bytes, E>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        let inspecting_stream = self.get_mut();
        match inspecting_stream.inner_stream.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(item) => {
                match &item {
                    Some(Ok(chunk)) => {
                        // Collect the chunk into our buffer.
                        inspecting_stream.collected.extend_from_slice(chunk);
                    }
                    None => {
                        // We reaached the end of the stream. Fire the callback.
                        if let Some(on_complete) = inspecting_stream.on_complete.take() {
                            let full_body = inspecting_stream.collected.split().freeze();
                            on_complete(full_body);
                        }
                    }
                    Some(Err(_)) => {
                        // We don't need to do anything on errors.
                    }
                }
                Poll::Ready(item)
            }
        }
    }
}
