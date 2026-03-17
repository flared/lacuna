use super::ByteInspector;
use super::Inspector;
use super::content_decoder::ContentDecoder;

/// A wrapper inspector that decompresses bytes before feeding them to the inner inspector.
pub struct DecodingInspector<T> {
    decoder: ContentDecoder,
    inner: ByteInspector<T>,
}

impl<T> std::fmt::Debug for DecodingInspector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodingInspector")
            .field("decoder", &self.decoder)
            .finish_non_exhaustive()
    }
}

impl<T> DecodingInspector<T> {
    pub fn wrap(inner: ByteInspector<T>, encoding: &str) -> ByteInspector<T>
    where
        T: Send + 'static,
    {
        let decoder = ContentDecoder::from_content_encoding(encoding);
        match decoder {
            Some(decoder) => Box::new(Self { decoder, inner }),
            None => {
                tracing::warn!("Unknown encoding: {encoding}");
                inner
            }
        }
    }
}

impl<T: Send> Inspector<&[u8]> for DecodingInspector<T> {
    type Output = T;

    fn feed(&mut self, chunk: &[u8]) {
        let decoded = self.decoder.decode(chunk);
        self.inner.feed(decoded);
        self.decoder.drain();
    }

    fn finish(self: Box<Self>) -> Result<T, anyhow::Error> {
        let mut this = *self;
        let remaining = this.decoder.finish();
        if !remaining.is_empty() {
            this.inner.feed(&remaining);
        }
        this.inner.finish()
    }
}
