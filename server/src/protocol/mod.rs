pub mod messages;
pub mod noise_transport;

pub use noise_transport::NoiseTransport;

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

/// Framing codec for C2 messages
pub struct C2Codec {
    inner: LengthDelimitedCodec,
}

impl C2Codec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::builder()
                .length_field_length(4)
                .max_frame_length(16 * 1024 * 1024) // 16MB max
                .new_codec(),
        }
    }
}

impl Decoder for C2Codec {
    type Item = BytesMut;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }
}

impl Encoder<Vec<u8>> for C2Codec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner.encode(item.into(), dst)
    }
}