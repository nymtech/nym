use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(crate) mod varint_async;
pub(crate) mod varint_bytes;

pub(crate) type Codec = SerdeCodec;

#[derive(Debug, Error)]
pub enum DecodingError {
    #[error("Decoding error: {0}")]
    DecodingError(#[from] serde_json::Error),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("Encoding error: {0}")]
    EncodingError(#[from] serde_json::Error),
}

/// Simple trait for encoding
pub(crate) trait EphemeraCodec {
    /// Encodes a message into a vector of bytes
    ///
    /// # Arguments
    ///
    /// * `data` - data to encode
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, EncodingError>` - encoded data or error
    ///
    /// # Errors
    ///
    /// * `EncodingError` - if encoding fails
    fn encode<M: Serialize>(data: &M) -> Result<Vec<u8>, EncodingError>;

    /// Decodes a message from a vector of bytes
    fn decode<M: for<'de> serde::Deserialize<'de>>(bytes: &[u8]) -> Result<M, DecodingError>;
}

pub(crate) struct SerdeCodec;

impl EphemeraCodec for SerdeCodec {
    fn encode<M: Serialize>(data: &M) -> Result<Vec<u8>, EncodingError> {
        let bytes = serde_json::to_vec(data)?;
        Ok(bytes)
    }

    fn decode<M: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<M, DecodingError> {
        let decoded = serde_json::from_slice(bytes)?;
        Ok(decoded)
    }
}

/// Trait which types can implement to provide their own encoding
pub trait Encode {
    /// Encodes itself into a vector of bytes
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, EncodingError>` - encoded data or error
    ///
    /// # Errors
    ///
    /// * `EncodingError` - if encoding fails
    fn encode(&self) -> Result<Vec<u8>, EncodingError>;
}

/// Trait which types can implement to provide their own decoding
pub trait Decode {
    type Output: for<'de> serde::Deserialize<'de>;

    /// Decodes itself from a vector of bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - bytes to decode
    ///
    /// # Returns
    ///
    /// * `Result<Self::Output, DecodingError>` - decoded data or error
    ///
    /// # Errors
    ///
    /// * `DecodingError` - if decoding fails
    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError>;
}

#[cfg(test)]
mod test {
    use crate::utilities::codec::EphemeraCodec;

    #[test]
    fn test_encode_decode() {
        let data = vec![1, 2, 3, 4, 5];
        let encoded = super::SerdeCodec::encode(&data).unwrap();
        let decoded = super::SerdeCodec::decode::<Vec<u8>>(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
