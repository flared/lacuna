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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    struct CollectingInspector {
        buf: Vec<u8>,
    }

    impl Inspector<&[u8]> for CollectingInspector {
        type Output = Vec<u8>;

        fn feed(&mut self, chunk: &[u8]) {
            self.buf.extend_from_slice(chunk);
        }

        fn finish(self: Box<Self>) -> Result<Vec<u8>, anyhow::Error> {
            Ok(self.buf)
        }
    }

    #[test]
    fn inner_inspector_receives_decoded_bytes() {
        let input = b"hello world";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        let compressed = encoder.finish().unwrap();

        let inner: ByteInspector<Vec<u8>> = Box::new(CollectingInspector { buf: Vec::new() });
        let mut inspector = DecodingInspector::wrap(inner, "gzip");

        for chunk in compressed.chunks(4) {
            inspector.feed(chunk);
        }

        let result = inspector.finish().unwrap();
        assert_eq!(result, input);
    }
}
