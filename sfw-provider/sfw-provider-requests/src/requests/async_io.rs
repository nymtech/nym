use crate::requests::serialization::{RequestDeserializer, RequestSerializer};
use crate::requests::{ProviderRequest, ProviderRequestError};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// TODO: way down the line, mostly for learning purposes, combine this with responses::async_io
// via procedural macros

impl<'a, R: AsyncRead + Unpin> Drop for TokioAsyncRequestReader<'a, R> {
    fn drop(&mut self) {
        println!("request reader drop");
    }
}

impl<'a, R: AsyncWrite + Unpin> Drop for TokioAsyncRequestWriter<'a, R> {
    fn drop(&mut self) {
        println!("request writer drop");
    }
}

// Ideally I would have used futures::AsyncRead for even more generic approach, but unfortunately
// tokio::io::AsyncRead differs from futures::AsyncRead
pub struct TokioAsyncRequestReader<'a, R: AsyncRead + Unpin> {
    max_allowed_len: usize,
    reader: &'a mut R,
}

impl<'a, R: AsyncRead + Unpin> TokioAsyncRequestReader<'a, R> {
    pub fn new(reader: &'a mut R, max_allowed_len: usize) -> Self {
        TokioAsyncRequestReader {
            reader,
            max_allowed_len,
        }
    }

    pub async fn try_read_request(&mut self) -> Result<ProviderRequest, ProviderRequestError> {
        let req_len = self.reader.read_u32().await?;
        if req_len == 0 {
            return Err(ProviderRequestError::RemoteConnectionClosed);
        }
        if req_len as usize > self.max_allowed_len {
            // TODO: should reader be drained?
            return Err(ProviderRequestError::TooLongRequestError);
        }

        let mut req_buf = Vec::with_capacity(req_len as usize);
        let mut chunk = self.reader.take(req_len as u64);

        if let Err(_) = chunk.read_to_end(&mut req_buf).await {
            return Err(ProviderRequestError::TooShortRequestError);
        };

        let parse_res = RequestDeserializer::new_with_len(req_len, &req_buf)?.try_to_parse();

        parse_res
    }
}

// Ideally I would have used futures::AsyncWrite for even more generic approach, but unfortunately
// tokio::io::AsyncWrite differs from futures::AsyncWrite
pub struct TokioAsyncRequestWriter<'a, W: AsyncWrite + Unpin> {
    writer: &'a mut W,
}

impl<'a, W: AsyncWrite + Unpin> TokioAsyncRequestWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        TokioAsyncRequestWriter { writer }
    }

    pub async fn try_write_request(&mut self, res: ProviderRequest) -> io::Result<()> {
        let res_bytes = RequestSerializer::new(res).into_bytes();
        self.writer.write_all(&res_bytes).await
    }
}
