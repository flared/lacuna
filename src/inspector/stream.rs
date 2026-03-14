use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::ByteInspector;

/// A stream wrapper that feeds chunks to an `Inspector<&[u8]>` as they pass through,
/// extracting output incrementally without buffering the full body.
pub struct InspectorStream<S, T> {
    inner_stream: Pin<Box<S>>,
    inspector: Option<ByteInspector<T>>,
}

impl<S: std::fmt::Debug, T> std::fmt::Debug for InspectorStream<S, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectorStream")
            .field("inner_stream", &self.inner_stream)
            .finish_non_exhaustive()
    }
}

impl<S, T> InspectorStream<S, T> {
    pub fn new(stream: S, inspector: ByteInspector<T>) -> Self {
        Self {
            inner_stream: Box::pin(stream),
            inspector: Some(inspector),
        }
    }
}

impl<S, T, E> Stream for InspectorStream<S, T>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    type Item = Result<Bytes, E>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        let this = self.get_mut();
        match this.inner_stream.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(item) => {
                match &item {
                    Some(Ok(chunk)) => {
                        if let Some(inspector) = &mut this.inspector {
                            inspector.feed(chunk);
                        }
                    }
                    None => {
                        if let Some(inspector) = this.inspector.take() {
                            let _ = inspector.finish();
                        }
                    }
                    Some(Err(_)) => {}
                }
                Poll::Ready(item)
            }
        }
    }
}
