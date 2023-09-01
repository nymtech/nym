use futures::{AsyncRead, AsyncWrite};
use futures_util::{AsyncReadExt, AsyncWriteExt};
use log::error;

#[allow(clippy::cast_possible_truncation)]

pub(crate) async fn write_length_prefixed<D: AsRef<[u8]>, I: AsyncWrite + Unpin>(
    io: &mut I,
    data: D,
) -> Result<(), std::io::Error> {
    write_varint(io, data.as_ref().len() as u32).await?;
    io.write_all(data.as_ref()).await?;
    io.flush().await?;

    Ok(())
}

async fn write_varint<I: AsyncWrite + Unpin>(io: &mut I, len: u32) -> Result<(), std::io::Error> {
    let mut len_data = unsigned_varint::encode::u32_buffer();
    let encoded_len = unsigned_varint::encode::u32(len, &mut len_data).len();
    io.write_all(&len_data[..encoded_len]).await?;

    Ok(())
}

async fn read_varint<I: AsyncRead + Unpin>(io: &mut I) -> Result<u32, std::io::Error> {
    let mut buffer = unsigned_varint::encode::u32_buffer();
    let mut buffer_len = 0;

    loop {
        //read 1 byte at time because we don't know how it compacted 32 bit integer
        io.read_exact(&mut buffer[buffer_len..=buffer_len]).await?;
        buffer_len += 1;
        match unsigned_varint::decode::u32(&buffer[..buffer_len]) {
            Ok((len, _)) => {
                return Ok(len);
            }
            Err(unsigned_varint::decode::Error::Overflow) => {
                error!("Invalid varint received");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid varint",
                ));
            }
            Err(unsigned_varint::decode::Error::Insufficient) => {
                continue;
            }
            Err(_) => {
                error!("Varint decoding error: #[non_exhaustive]");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid varint",
                ));
            }
        }
    }
}

pub(crate) async fn read_length_prefixed<I: AsyncRead + Unpin>(
    io: &mut I,
    max_size: u32,
) -> Result<Vec<u8>, std::io::Error> {
    let len = read_varint(io).await?;
    if len > max_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Message too large",
        ));
    }

    let mut buf = vec![0; len as usize];
    io.read_exact(&mut buf).await?;
    Ok(buf)
}

#[cfg(test)]
mod test {

    use futures_util::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_read_write() {
        let data = "hello world".to_string();

        let mut buf = Vec::with_capacity(data.len() + 1);
        let mut cursor = Cursor::new(&mut buf);

        write_length_prefixed(&mut cursor, data.as_bytes())
            .await
            .unwrap();
        cursor.set_position(0);

        let vec = read_length_prefixed(&mut cursor, 100).await.unwrap();
        let result = String::from_utf8(vec).unwrap();

        assert_eq!(result, data);
    }
}
