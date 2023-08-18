use bytes::{Buf, BytesMut};

use thiserror::Error;
use unsigned_varint::{decode, encode};

#[derive(Debug, Error)]
pub(crate) enum VarintError {
    #[error("InvalidInput: {0}")]
    InvalidInput(String),
    #[error("varint error: {0}")]
    Varint(#[from] decode::Error),
    #[error("TooLarge")]
    TooLarge,
}

#[allow(clippy::cast_possible_truncation)]

pub(crate) fn write_length_prefixed<D: AsRef<[u8]>>(dst: &mut BytesMut, data: D) {
    write_varint(dst, data.as_ref().len() as u32);
    dst.extend_from_slice(data.as_ref());
}

fn write_varint(dst: &mut BytesMut, len: u32) {
    let mut len_data = encode::u32_buffer();
    let encoded_len = encode::u32(len, &mut len_data).len();
    dst.extend_from_slice(&len_data[..encoded_len]);
}

pub(crate) fn read_length_prefixed(
    bytes: &mut BytesMut,
    max_size: u32,
) -> Result<Option<Vec<u8>>, VarintError> {
    let len = read_varint(bytes)?;
    if len > max_size {
        return Err(VarintError::TooLarge);
    }

    if bytes.remaining() < len as usize {
        return Ok(None);
    }

    let vec = bytes.to_vec();
    bytes.advance(len as usize);
    Ok(Some(vec))
}

fn read_varint(bytes: &mut BytesMut) -> Result<u32, VarintError> {
    let mut buffer = encode::u32_buffer();

    for (i, byte) in bytes.iter().enumerate() {
        buffer[i] = *byte;
        match decode::u32(&buffer[..i]) {
            Ok((len, _)) => {
                bytes.advance(i);
                return Ok(len);
            }
            Err(decode::Error::Insufficient) => continue,
            Err(err) => Err(err)?,
        }
    }

    Err(VarintError::InvalidInput(
        "Unable to read varint".to_string(),
    ))
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn test_read_write() {
        let data = "hello world".to_string();

        let mut encoded = BytesMut::with_capacity(0);
        write_length_prefixed(&mut encoded, data.clone());
        let vec = read_length_prefixed(&mut encoded, 100).unwrap().unwrap();
        let result = String::from_utf8(vec).unwrap();

        assert_eq!(result, data);
        assert_eq!(encoded.remaining(), 0);
    }
}
