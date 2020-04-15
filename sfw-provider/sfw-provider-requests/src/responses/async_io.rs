use crate::responses::serialization::{ResponseDeserializer, ResponseSerializer};
use crate::responses::{ProviderResponse, ProviderResponseError};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// TODO: way down the line, mostly for learning purposes, combine this with requests::async_io
// via procedural macros

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
            // TODO: should reader be drained or just assume caller will close the
            // underlying reader and/or deal with the issue itself?
            return Err(ProviderResponseError::TooLongResponseError);
        }

        let mut res_buf = vec![0; res_len as usize];
        if let Err(e) = self.reader.read_exact(&mut res_buf).await {
            return Err(ProviderResponseError::IOError(e));
        }

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

#[cfg(test)]
mod response_writer {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use crate::responses::{FailureResponse, PullResponse, RegisterResponse};

    // TODO: what else to test here?

    #[test]
    fn writes_all_bytes_to_underlying_writer_for_register_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);
        let expected_bytes = ResponseSerializer::new(register_response.clone().into()).into_bytes();

        let mut writer = Vec::new();

        let mut response_writer = TokioAsyncResponseWriter::new(&mut writer);
        rt.block_on(response_writer.try_write_response(register_response.into()))
            .unwrap();

        // to finish the mutable borrow since we don't need response_writer anymore anyway
        drop(response_writer);

        assert_eq!(writer, expected_bytes);
    }

    #[test]
    fn writes_all_bytes_to_underlying_writer_for_pull_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let msg1 = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ];
        let msg2 = vec![1, 2, 3, 4, 5, 6, 7];
        let pull_response = PullResponse::new(vec![msg1, msg2]);

        let expected_bytes = ResponseSerializer::new(pull_response.clone().into()).into_bytes();

        let mut writer = Vec::new();

        let mut response_writer = TokioAsyncResponseWriter::new(&mut writer);
        rt.block_on(response_writer.try_write_response(pull_response.into()))
            .unwrap();

        // to finish the mutable borrow since we don't need response_writer anymore anyway
        drop(response_writer);

        assert_eq!(writer, expected_bytes);
    }

    #[test]
    fn writes_all_bytes_to_underlying_writer_for_failure_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let msg1 = "hello nym";
        let failure_response = FailureResponse::new(msg1);

        let expected_bytes = ResponseSerializer::new(failure_response.clone().into()).into_bytes();

        let mut writer = Vec::new();

        let mut response_writer = TokioAsyncResponseWriter::new(&mut writer);
        rt.block_on(response_writer.try_write_response(failure_response.into()))
            .unwrap();

        // to finish the mutable borrow since we don't need response_writer anymore anyway
        drop(response_writer);

        assert_eq!(writer, expected_bytes);
    }
}

#[cfg(test)]
mod response_reader {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use crate::responses::{PullResponse, RegisterResponse, ResponseKind};
    use std::io::Cursor;
    use std::time;

    #[test]
    fn correctly_reads_valid_register_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);
        let serialized_bytes =
            ResponseSerializer::new(register_response.clone().into()).into_bytes();

        let mut reader = Cursor::new(serialized_bytes);
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut reader, u32::max_value() as usize);

        let read_response = rt.block_on(response_reader.try_read_response()).unwrap();
        match read_response {
            ProviderResponse::Register(req) => assert_eq!(register_response, req),
            _ => panic!("read incorrect response!"),
        }
    }

    #[test]
    fn correctly_reads_valid_pull_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let msg1 = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ];
        let msg2 = vec![1, 2, 3, 4, 5, 6, 7];
        let pull_response = PullResponse::new(vec![msg1, msg2]);
        let serialized_bytes = ResponseSerializer::new(pull_response.clone().into()).into_bytes();

        let mut reader = Cursor::new(serialized_bytes);
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut reader, u32::max_value() as usize);

        let read_response = rt.block_on(response_reader.try_read_response()).unwrap();
        match read_response {
            ProviderResponse::Pull(req) => assert_eq!(pull_response, req),
            _ => panic!("read incorrect response!"),
        }
    }

    #[test]
    fn correctly_reads_valid_register_response_even_if_more_random_bytes_follow() {
        // note that if read was called again, it would have failed

        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);
        let serialized_bytes =
            ResponseSerializer::new(register_response.clone().into()).into_bytes();

        let serialized_bytes_with_garbage: Vec<_> = serialized_bytes
            .into_iter()
            .chain(vec![1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter())
            .collect();

        let mut reader = Cursor::new(serialized_bytes_with_garbage);
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut reader, u32::max_value() as usize);

        let read_response = rt.block_on(response_reader.try_read_response()).unwrap();
        match read_response {
            ProviderResponse::Register(req) => assert_eq!(register_response, req),
            _ => panic!("read incorrect response!"),
        }
    }

    #[test]
    fn correctly_reads_two_consecutive_responses() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let msg1 = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ];
        let msg2 = vec![1, 2, 3, 4, 5, 6, 7];
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);

        let pull_response = PullResponse::new(vec![msg1, msg2]);
        let register_response = RegisterResponse::new(auth_token);

        let register_serialized_bytes =
            ResponseSerializer::new(register_response.clone().into()).into_bytes();
        let pull_serialized_bytes =
            ResponseSerializer::new(pull_response.clone().into()).into_bytes();

        let combined_responses: Vec<_> = register_serialized_bytes
            .into_iter()
            .chain(pull_serialized_bytes.into_iter())
            .collect();

        let mut reader = Cursor::new(combined_responses);
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut reader, u32::max_value() as usize);

        let first_read_response = rt.block_on(response_reader.try_read_response()).unwrap();
        match first_read_response {
            ProviderResponse::Register(req) => assert_eq!(register_response, req),
            _ => panic!("read incorrect response!"),
        }

        let second_read_response = rt.block_on(response_reader.try_read_response()).unwrap();
        match second_read_response {
            ProviderResponse::Pull(req) => assert_eq!(pull_response, req),
            _ => panic!("read incorrect response!"),
        }
    }

    #[test]
    fn correctly_reads_valid_response_even_if_written_with_delay() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);
        let serialized_bytes =
            ResponseSerializer::new(register_response.clone().into()).into_bytes();

        let (first_half, second_half) = serialized_bytes.split_at(30); // 30 is an arbitrary value

        let mut mock = tokio_test::io::Builder::new()
            .read(&first_half)
            .wait(time::Duration::from_millis(300))
            .read(&second_half)
            .build();

        let mut response_reader =
            TokioAsyncResponseReader::new(&mut mock, u32::max_value() as usize);

        let read_response = rt.block_on(response_reader.try_read_response()).unwrap();
        match read_response {
            ProviderResponse::Register(req) => assert_eq!(register_response, req),
            _ => panic!("read incorrect response!"),
        }
    }

    #[test]
    fn fails_to_read_invalid_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let mut invalid_response = 9u32.to_be_bytes().to_vec();
        invalid_response.push(ResponseKind::Register as u8); // to have a 'valid' kind
        invalid_response.push(0);
        invalid_response.push(1);
        invalid_response.push(2);
        invalid_response.push(3);
        invalid_response.push(4);
        invalid_response.push(5);
        invalid_response.push(6);
        invalid_response.push(7);

        let mut reader = Cursor::new(invalid_response);
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut reader, u32::max_value() as usize);

        assert!(rt.block_on(response_reader.try_read_response()).is_err());
    }

    #[test]
    fn fails_to_read_too_long_response() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);
        let serialized_bytes =
            ResponseSerializer::new(register_response.clone().into()).into_bytes();
        let serialized_bytes_len = serialized_bytes.len();

        let mut reader = Cursor::new(serialized_bytes);
        // note our reader accepts fewer bytes than what we have
        let mut response_reader =
            TokioAsyncResponseReader::new(&mut reader, serialized_bytes_len - 10);

        assert!(rt.block_on(response_reader.try_read_response()).is_err());
    }
}
