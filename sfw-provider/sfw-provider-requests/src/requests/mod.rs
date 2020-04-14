use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
use sphinx::constants::DESTINATION_ADDRESS_LENGTH;
use sphinx::route::DestinationAddressBytes;
use std::convert::TryFrom;
use std::io;
use std::io::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub enum ProviderRequestError {
    MarshalError,
    UnmarshalError,
    UnmarshalErrorInvalidKind,
    UnmarshalErrorInvalidLength,
    TooLongRequestError,
    TooShortRequestError,
    IOError(io::Error),
    RemoteConnectionClosed,
}

impl From<io::Error> for ProviderRequestError {
    fn from(e: Error) -> Self {
        ProviderRequestError::IOError(e)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum RequestKind {
    Pull = 1,
    Register = 2,
}

impl TryFrom<u8> for RequestKind {
    type Error = ProviderRequestError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (RequestKind::Pull as u8) => Ok(Self::Pull),
            _ if value == (RequestKind::Register as u8) => Ok(Self::Register),
            _ => Err(Self::Error::UnmarshalErrorInvalidKind),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderRequest {
    Pull(PullRequest),
    Register(RegisterRequest),
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

pub struct RequestDeserializer<'a> {
    kind: RequestKind,
    data: &'a [u8],
}

impl<'a> RequestDeserializer<'a> {
    // perform initial parsing
    pub fn new(raw_bytes: &'a [u8]) -> Result<Self, ProviderRequestError> {
        if raw_bytes.len() < 1 + 4 {
            Err(ProviderRequestError::UnmarshalErrorInvalidLength)
        } else {
            let data_len =
                u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]]);
            let kind = RequestKind::try_from(raw_bytes[4])?;
            let data = &raw_bytes[4..];

            if data.len() != data_len as usize {
                Err(ProviderRequestError::UnmarshalErrorInvalidLength)
            } else {
                Ok(RequestDeserializer { kind, data })
            }
        }
    }

    pub fn new_with_len(len: u32, raw_bytes: &'a [u8]) -> Result<Self, ProviderRequestError> {
        if raw_bytes.len() != len as usize {
            Err(ProviderRequestError::UnmarshalErrorInvalidLength)
        } else {
            let kind = RequestKind::try_from(raw_bytes[0])?;
            let data = &raw_bytes[1..];
            Ok(RequestDeserializer { kind, data })
        }
    }

    pub fn get_kind(&self) -> RequestKind {
        self.kind
    }

    pub fn get_data(&self) -> &'a [u8] {
        self.data
    }

    pub fn try_to_parse(self) -> Result<ProviderRequest, ProviderRequestError> {
        match self.get_kind() {
            RequestKind::Pull => Ok(ProviderRequest::Pull(PullRequest::try_from_bytes(
                self.data,
            )?)),
            RequestKind::Register => Ok(ProviderRequest::Register(
                RegisterRequest::try_from_bytes(self.data)?,
            )),
        }
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

pub struct RequestSerializer {
    req: ProviderRequest,
}

impl RequestSerializer {
    pub fn new(req: ProviderRequest) -> Self {
        RequestSerializer { req }
    }

    /// Serialized requests in general have the following structure:
    /// follows: 4 byte len (be u32) || 1-byte kind prefix || request-specific data
    pub fn into_bytes(self) -> Vec<u8> {
        let (kind, req_bytes) = match self.req {
            ProviderRequest::Pull(req) => (req.get_kind(), req.to_bytes()),
            ProviderRequest::Register(req) => (req.get_kind(), req.to_bytes()),
        };
        let req_len = req_bytes.len() as u32 + 1; // 1 is to accommodate for 'kind'
        let req_len_bytes = req_len.to_be_bytes();
        req_len_bytes
            .iter()
            .cloned()
            .chain(std::iter::once(kind as u8))
            .chain(req_bytes.into_iter())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PullRequest {
    pub auth_token: AuthToken,
    pub destination_address: sphinx::route::DestinationAddressBytes,
}

impl PullRequest {
    pub fn new(
        destination_address: sphinx::route::DestinationAddressBytes,
        auth_token: AuthToken,
    ) -> Self {
        PullRequest {
            auth_token,
            destination_address,
        }
    }

    pub fn get_kind(&self) -> RequestKind {
        RequestKind::Pull
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.destination_address
            .to_bytes()
            .iter()
            .cloned()
            .chain(self.auth_token.as_bytes().iter().cloned())
            .collect()
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestError> {
        if bytes.len() != DESTINATION_ADDRESS_LENGTH + AUTH_TOKEN_SIZE {
            return Err(ProviderRequestError::UnmarshalErrorInvalidLength);
        }

        let mut destination_address = [0u8; DESTINATION_ADDRESS_LENGTH];
        destination_address.copy_from_slice(&bytes[..DESTINATION_ADDRESS_LENGTH]);

        let mut auth_token = [0u8; AUTH_TOKEN_SIZE];
        auth_token.copy_from_slice(&bytes[DESTINATION_ADDRESS_LENGTH..]);

        Ok(PullRequest {
            auth_token: AuthToken::from_bytes(auth_token),
            destination_address: DestinationAddressBytes::from_bytes(destination_address),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterRequest {
    pub destination_address: DestinationAddressBytes,
}

impl RegisterRequest {
    pub fn new(destination_address: DestinationAddressBytes) -> Self {
        RegisterRequest {
            destination_address,
        }
    }

    pub fn get_kind(&self) -> RequestKind {
        RequestKind::Register
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.destination_address
            .to_bytes()
            .iter()
            .cloned()
            .collect()
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestError> {
        if bytes.len() != DESTINATION_ADDRESS_LENGTH {
            return Err(ProviderRequestError::UnmarshalErrorInvalidLength);
        }

        let mut destination_address = [0u8; DESTINATION_ADDRESS_LENGTH];
        destination_address.copy_from_slice(&bytes[..DESTINATION_ADDRESS_LENGTH]);

        Ok(RegisterRequest {
            destination_address: DestinationAddressBytes::from_bytes(destination_address),
        })
    }
}

#[cfg(test)]
mod creating_pull_request {
    use super::*;

    #[test]
    fn it_is_possible_to_recover_it_from_bytes() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = [1u8; AUTH_TOKEN_SIZE];
        let pull_request = PullRequest::new(address.clone(), AuthToken::from_bytes(auth_token));
        let bytes = pull_request.to_bytes();

        let recovered = PullRequest::try_from_bytes(&bytes).unwrap();
        assert_eq!(recovered, pull_request);
    }
}

#[cfg(test)]
mod creating_register_request {
    use super::*;

    #[test]
    fn it_is_possible_to_recover_it_from_bytes() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address.clone());
        let bytes = register_request.to_bytes();

        let recovered = RegisterRequest::try_from_bytes(&bytes).unwrap();
        assert_eq!(recovered, register_request);
    }
}
