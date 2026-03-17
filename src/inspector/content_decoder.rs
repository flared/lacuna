use std::io::Write;

use flate2::write::{DeflateDecoder, GzDecoder, ZlibDecoder};

/// Decodes compressed content using a push-based (Write) interface.
#[derive(Debug)]
pub enum ContentDecoder {
    Gzip(GzDecoder<Vec<u8>>),
    Deflate(DeflateDecoder<Vec<u8>>),
    Zlib(ZlibDecoder<Vec<u8>>),
}

impl ContentDecoder {
    pub fn from_content_encoding(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "gzip" | "x-gzip" => Some(Self::Gzip(GzDecoder::new(Vec::new()))),
            "deflate" => Some(Self::Deflate(DeflateDecoder::new(Vec::new()))),
            "zlib" => Some(Self::Zlib(ZlibDecoder::new(Vec::new()))),
            _ => None,
        }
    }

    /// Decode a chunk of compressed bytes, returning the decompressed output.
    pub fn decode<'a>(&'a mut self, chunk: &'a [u8]) -> &'a [u8] {
        match self {
            Self::Gzip(decoder) => {
                let _ = decoder.write_all(chunk);
                let _ = decoder.flush();
                let buf = decoder.get_ref();
                buf.as_slice()
            }
            Self::Deflate(decoder) => {
                let _ = decoder.write_all(chunk);
                let _ = decoder.flush();
                let buf = decoder.get_ref();
                buf.as_slice()
            }
            Self::Zlib(decoder) => {
                let _ = decoder.write_all(chunk);
                let _ = decoder.flush();
                let buf = decoder.get_ref();
                buf.as_slice()
            }
        }
    }

    /// Drain the internal buffer after reading the decoded output.
    /// Must be called after each `decode` to clear the buffer.
    pub fn drain(&mut self) {
        match self {
            Self::Gzip(decoder) => decoder.get_mut().clear(),
            Self::Deflate(decoder) => decoder.get_mut().clear(),
            Self::Zlib(decoder) => decoder.get_mut().clear(),
        }
    }

    /// Finalize the decoder and return any remaining bytes.
    pub fn finish(self) -> Vec<u8> {
        match self {
            Self::Gzip(decoder) => decoder.finish().unwrap_or_default(),
            Self::Deflate(decoder) => decoder.finish().unwrap_or_default(),
            Self::Zlib(decoder) => decoder.finish().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
    use std::io::Write;

    fn compress_gzip(data: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn compress_deflate(data: &[u8]) -> Vec<u8> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn compress_zlib(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn from_content_encoding() {
        assert!(matches!(
            ContentDecoder::from_content_encoding("gzip"),
            Some(ContentDecoder::Gzip(_))
        ));
        assert!(matches!(
            ContentDecoder::from_content_encoding("x-gzip"),
            Some(ContentDecoder::Gzip(_))
        ));
        assert!(matches!(
            ContentDecoder::from_content_encoding("deflate"),
            Some(ContentDecoder::Deflate(_))
        ));
        assert!(matches!(
            ContentDecoder::from_content_encoding("zlib"),
            Some(ContentDecoder::Zlib(_))
        ));

        // Case insensitive
        assert!(matches!(
            ContentDecoder::from_content_encoding("GZIP"),
            Some(ContentDecoder::Gzip(_))
        ));
        assert!(matches!(
            ContentDecoder::from_content_encoding(" Gzip "),
            Some(ContentDecoder::Gzip(_))
        ));
        assert!(matches!(
            ContentDecoder::from_content_encoding("DEFLATE"),
            Some(ContentDecoder::Deflate(_))
        ));
        assert!(matches!(
            ContentDecoder::from_content_encoding(" Zlib "),
            Some(ContentDecoder::Zlib(_))
        ));

        // Unknown
        assert!(ContentDecoder::from_content_encoding("br").is_none());
        assert!(ContentDecoder::from_content_encoding("identity").is_none());
        assert!(ContentDecoder::from_content_encoding("").is_none());
    }

    #[test]
    fn decode_chunked_and_finish() {
        let input = b"hello world, this is a longer string for chunked decoding";
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("gzip", compress_gzip(input)),
            ("deflate", compress_deflate(input)),
            ("zlib", compress_zlib(input)),
        ];

        for (encoding, compressed) in cases {
            let mut decoder = ContentDecoder::from_content_encoding(encoding).unwrap();

            let mut result = Vec::new();
            for chunk in compressed.chunks(4) {
                let output = decoder.decode(chunk);
                result.extend_from_slice(output);
                decoder.drain();
            }

            assert_eq!(result, input, "failed for encoding: {encoding}");

            let remaining = decoder.finish();
            assert!(
                remaining.is_empty(),
                "finish not empty for encoding: {encoding}"
            );
        }
    }
}
