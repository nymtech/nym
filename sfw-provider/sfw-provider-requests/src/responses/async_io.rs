use crate::responses::serialization::{ResponseDeserializer, ResponseSerializer};
use crate::responses::{ProviderResponse, ProviderResponseError};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// TODO: way down the line, mostly for learning purposes, combine this with requests::async_io
// via procedural macros

impl<'a, R: AsyncRead + Unpin> Drop for TokioAsyncResponseReader<'a, R> {
    fn drop(&mut self) {
        println!("response reader drop");
    }
}

impl<'a, R: AsyncWrite + Unpin> Drop for TokioAsyncResponseWriter<'a, R> {
    fn drop(&mut self) {
        println!("response writer drop");
    }
}

// Ideally I would have used futures::AsyncRead for even more generic approach, but unfortunately
// tokio::io::AsyncRead differs from futures::AsyncRead
pub struct TokioAsyncResponseReader<'a, R: AsyncRead + Unpin> {
    max_allowed_len: usize,
    reader: &'a mut R,
}

impl<'a, R: AsyncRead + Unpin> TokioAsyncResponseReader<'a, R> {
    pub fn new(reader: &'a mut R, max_allowed_len: usize) -> Self {
        TokioAsyncResponseReader {
            reader,
            max_allowed_len,
        }
    }

    pub async fn try_read_response(&mut self) -> Result<ProviderResponse, ProviderResponseError> {
        let res_len = self.reader.read_u32().await?;
        if res_len == 0 {
            return Err(ProviderResponseError::RemoteConnectionClosed);
        }
        if res_len as usize > self.max_allowed_len {
            // TODO: should reader be drained?
            return Err(ProviderResponseError::TooLongResponseError);
        }

        let mut res_buf = Vec::with_capacity(res_len as usize);
        let mut chunk = self.reader.take(res_len as u64);

        if let Err(_) = chunk.read_to_end(&mut res_buf).await {
            return Err(ProviderResponseError::TooShortResponseError);
        };

        ResponseDeserializer::new_with_len(res_len, &res_buf)?.try_to_parse()
    }
}

// Ideally I would have used futures::AsyncWrite for even more generic approach, but unfortunately
// tokio::io::AsyncWrite differs from futures::AsyncWrite
pub struct TokioAsyncResponseWriter<'a, W: AsyncWrite + Unpin> {
    writer: &'a mut W,
}

impl<'a, W: AsyncWrite + Unpin> TokioAsyncResponseWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        TokioAsyncResponseWriter { writer }
    }

    pub async fn try_write_response(&mut self, res: ProviderResponse) -> io::Result<()> {
        let res_bytes = ResponseSerializer::new(res).into_bytes();
        self.writer.write_all(&res_bytes).await
    }
}
