use crate::requests::serialization::{RequestDeserializer, RequestSerializer};
use crate::requests::{ProviderRequest, ProviderRequestError};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// TODO: way down the line, mostly for learning purposes, combine this with responses::async_io
// via procedural macros

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
            // TODO: should reader be drained or just assume caller will close the
            // underlying reader and/or deal with the issue itself?
            return Err(ProviderRequestError::TooLongRequestError);
        }

        let mut req_buf = vec![0; req_len as usize];
        if let Err(e) = self.reader.read_exact(&mut req_buf).await {
            return Err(ProviderRequestError::IOError(e));
        }

        RequestDeserializer::new_with_len(req_len, &req_buf)?.try_to_parse()
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

#[cfg(test)]
mod request_writer {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use crate::requests::{PullRequest, RegisterRequest};
    use sphinx::route::DestinationAddressBytes;

    // TODO: what else to test here?

    #[test]
    fn writes_all_bytes_to_underlying_writer_for_register_request() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);
        let expected_bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();

        let mut writer = Vec::new();

        let mut request_writer = TokioAsyncRequestWriter::new(&mut writer);
        rt.block_on(request_writer.try_write_request(register_request.into()))
            .unwrap();

        // to finish the mutable borrow since we don't need request_writer anymore anyway
        drop(request_writer);

        assert_eq!(writer, expected_bytes);
    }

    #[test]
    fn writes_all_bytes_to_underlying_writer_for_pull_request() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let pull_request = PullRequest::new(address, auth_token);
        let expected_bytes = RequestSerializer::new(pull_request.clone().into()).into_bytes();

        let mut writer = Vec::new();

        let mut request_writer = TokioAsyncRequestWriter::new(&mut writer);
        rt.block_on(request_writer.try_write_request(pull_request.into()))
            .unwrap();

        // to finish the mutable borrow since we don't need request_writer anymore anyway
        drop(request_writer);

        assert_eq!(writer, expected_bytes);
    }
}

#[cfg(test)]
mod request_reader {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use crate::requests::{PullRequest, RegisterRequest, RequestKind};
    use sphinx::route::DestinationAddressBytes;
    use std::io::Cursor;
    use std::time;

    #[test]
    fn correctly_reads_valid_register_request() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);
        let serialized_bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();

        let mut reader = Cursor::new(serialized_bytes);
        let mut request_reader =
            TokioAsyncRequestReader::new(&mut reader, u32::max_value() as usize);

        let read_request = rt.block_on(request_reader.try_read_request()).unwrap();
        match read_request {
            ProviderRequest::Register(req) => assert_eq!(register_request, req),
            _ => panic!("read incorrect request!"),
        }
    }

    #[test]
    fn correctly_reads_valid_pull_request() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let pull_request = PullRequest::new(address, auth_token);
        let serialized_bytes = RequestSerializer::new(pull_request.clone().into()).into_bytes();

        let mut reader = Cursor::new(serialized_bytes);
        let mut request_reader =
            TokioAsyncRequestReader::new(&mut reader, u32::max_value() as usize);

        let read_request = rt.block_on(request_reader.try_read_request()).unwrap();
        match read_request {
            ProviderRequest::Pull(req) => assert_eq!(pull_request, req),
            _ => panic!("read incorrect request!"),
        }
    }

    #[test]
    fn correctly_reads_valid_register_request_even_if_more_random_bytes_follow() {
        // note that if read was called again, it would have failed

        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);
        let serialized_bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();

        let serialized_bytes_with_garbage: Vec<_> = serialized_bytes
            .into_iter()
            .chain(vec![1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter())
            .collect();

        let mut reader = Cursor::new(serialized_bytes_with_garbage);
        let mut request_reader =
            TokioAsyncRequestReader::new(&mut reader, u32::max_value() as usize);

        let read_request = rt.block_on(request_reader.try_read_request()).unwrap();
        match read_request {
            ProviderRequest::Register(req) => assert_eq!(register_request, req),
            _ => panic!("read incorrect request!"),
        }
    }

    #[test]
    fn correctly_reads_two_consecutive_requests() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);

        let pull_request = PullRequest::new(address.clone(), auth_token);
        let register_request = RegisterRequest::new(address);

        let register_serialized_bytes =
            RequestSerializer::new(register_request.clone().into()).into_bytes();
        let pull_serialized_bytes =
            RequestSerializer::new(pull_request.clone().into()).into_bytes();

        let combined_requests: Vec<_> = register_serialized_bytes
            .into_iter()
            .chain(pull_serialized_bytes.into_iter())
            .collect();

        let mut reader = Cursor::new(combined_requests);
        let mut request_reader =
            TokioAsyncRequestReader::new(&mut reader, u32::max_value() as usize);

        let first_read_request = rt.block_on(request_reader.try_read_request()).unwrap();
        match first_read_request {
            ProviderRequest::Register(req) => assert_eq!(register_request, req),
            _ => panic!("read incorrect request!"),
        }

        let second_read_request = rt.block_on(request_reader.try_read_request()).unwrap();
        match second_read_request {
            ProviderRequest::Pull(req) => assert_eq!(pull_request, req),
            _ => panic!("read incorrect request!"),
        }
    }

    #[test]
    fn correctly_reads_valid_request_even_if_written_with_delay() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);
        let serialized_bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();

        let (first_half, second_half) = serialized_bytes.split_at(30); // 30 is an arbitrary value

        let mut mock = tokio_test::io::Builder::new()
            .read(&first_half)
            .wait(time::Duration::from_millis(300))
            .read(&second_half)
            .build();

        let mut request_reader = TokioAsyncRequestReader::new(&mut mock, u32::max_value() as usize);

        let read_request = rt.block_on(request_reader.try_read_request()).unwrap();
        match read_request {
            ProviderRequest::Register(req) => assert_eq!(register_request, req),
            _ => panic!("read incorrect request!"),
        }
    }

    #[test]
    fn fails_to_read_invalid_request() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let mut invalid_request = 9u32.to_be_bytes().to_vec();
        invalid_request.push(RequestKind::Register as u8); // to have a 'valid' kind
        invalid_request.push(0);
        invalid_request.push(1);
        invalid_request.push(2);
        invalid_request.push(3);
        invalid_request.push(4);
        invalid_request.push(5);
        invalid_request.push(6);
        invalid_request.push(7);

        let mut reader = Cursor::new(invalid_request);
        let mut request_reader =
            TokioAsyncRequestReader::new(&mut reader, u32::max_value() as usize);

        assert!(rt.block_on(request_reader.try_read_request()).is_err());
    }

    #[test]
    fn fails_to_read_too_long_request() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);
        let serialized_bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();
        let serialized_bytes_len = serialized_bytes.len();

        let mut reader = Cursor::new(serialized_bytes);
        // note our reader accepts fewer bytes than what we have
        let mut request_reader =
            TokioAsyncRequestReader::new(&mut reader, serialized_bytes_len - 10);

        assert!(rt.block_on(request_reader.try_read_request()).is_err());
    }
}
