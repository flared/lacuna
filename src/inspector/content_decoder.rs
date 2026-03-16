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
