use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
use sphinx::constants::DESTINATION_ADDRESS_LENGTH;
use sphinx::route::DestinationAddressBytes;
use std::convert::TryFrom;
use std::io;
use std::io::Error;

pub mod async_io;
pub mod serialization;

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
