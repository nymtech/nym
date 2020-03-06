use crate::AuthToken;
use sphinx::route::DestinationAddressBytes;

const PULL_REQUEST_MESSAGE_PREFIX: [u8; 2] = [1, 0];
const REGISTER_MESSAGE_PREFIX: [u8; 2] = [0, 1];

// TODO: how to do it more nicely, considering all sfw-provider-requests implement same trait that is exercised here?
#[derive(Debug)]
pub enum ProviderRequests {
    PullMessages(PullRequest),
    Register(RegisterRequest),
}

impl ProviderRequests {
    pub fn to_bytes(&self) -> Vec<u8> {
        use ProviderRequests::*;
        match self {
            PullMessages(pr) => pr.to_bytes(),
            Register(pr) => pr.to_bytes(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestError> {
        use ProviderRequests::*;
        if bytes.len() < 2 {
            return Err(ProviderRequestError::UnmarshalError);
        }
        let mut received_prefix = [0; 2];
        received_prefix.copy_from_slice(&bytes[..2]);
        match received_prefix {
            PULL_REQUEST_MESSAGE_PREFIX => Ok(PullMessages(PullRequest::from_bytes(bytes)?)),
            REGISTER_MESSAGE_PREFIX => Ok(Register(RegisterRequest::from_bytes(bytes)?)),
            _ => Err(ProviderRequestError::UnmarshalErrorIncorrectPrefix),
        }
    }
}

#[derive(Debug)]
pub enum ProviderRequestError {
    MarshalError,
    UnmarshalError,
    UnmarshalErrorIncorrectPrefix,
}

pub trait ProviderRequest
where
    Self: Sized,
{
    fn get_prefix() -> [u8; 2];
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestError>;
}

#[derive(Debug)]
pub struct PullRequest {
    // TODO: public keys, signatures, tokens, etc. basically some kind of authentication bs
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
}

impl ProviderRequest for PullRequest {
    fn get_prefix() -> [u8; 2] {
        PULL_REQUEST_MESSAGE_PREFIX
    }

    fn to_bytes(&self) -> Vec<u8> {
        Self::get_prefix()
            .to_vec()
            .into_iter()
            .chain(self.destination_address.to_bytes().iter().cloned())
            .chain(self.auth_token.0.iter().cloned())
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestError> {
        if bytes.len() != 2 + 32 + 32 {
            return Err(ProviderRequestError::UnmarshalError);
        }

        let mut received_prefix = [0u8; 2];
        received_prefix.copy_from_slice(&bytes[..2]);
        if received_prefix != Self::get_prefix() {
            return Err(ProviderRequestError::UnmarshalErrorIncorrectPrefix);
        }

        let mut destination_address = [0u8; 32];
        destination_address.copy_from_slice(&bytes[2..34]);

        let mut auth_token = [0u8; 32];
        auth_token.copy_from_slice(&bytes[34..]);

        Ok(PullRequest {
            auth_token: AuthToken::from_bytes(auth_token),
            destination_address: DestinationAddressBytes::from_bytes(destination_address),
        })
    }
}

#[derive(Debug)]
pub struct RegisterRequest {
    pub destination_address: DestinationAddressBytes,
}

impl RegisterRequest {
    pub fn new(destination_address: DestinationAddressBytes) -> Self {
        RegisterRequest {
            destination_address,
        }
    }
}

impl ProviderRequest for RegisterRequest {
    fn get_prefix() -> [u8; 2] {
        REGISTER_MESSAGE_PREFIX
    }

    fn to_bytes(&self) -> Vec<u8> {
        Self::get_prefix()
            .to_vec()
            .into_iter()
            .chain(self.destination_address.to_bytes().iter().cloned())
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestError> {
        if bytes.len() != 2 + 32 {
            return Err(ProviderRequestError::UnmarshalError);
        }

        let mut received_prefix = [0u8; 2];
        received_prefix.copy_from_slice(&bytes[..2]);
        if received_prefix != Self::get_prefix() {
            return Err(ProviderRequestError::UnmarshalErrorIncorrectPrefix);
        }

        let mut destination_address = [0u8; 32];
        destination_address.copy_from_slice(&bytes[2..]);

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
        let auth_token = [1u8; 32];
        let pull_request = PullRequest::new(address.clone(), AuthToken(auth_token));
        let bytes = pull_request.to_bytes();

        let recovered = PullRequest::from_bytes(&bytes).unwrap();
        assert_eq!(address, recovered.destination_address);
        assert_eq!(AuthToken(auth_token), recovered.auth_token);
    }

    #[test]
    fn it_is_possible_to_recover_it_from_bytes_with_enum_wrapper() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = [1u8; 32];
        let pull_request = PullRequest::new(address.clone(), AuthToken(auth_token));
        let bytes = pull_request.to_bytes();

        let recovered = ProviderRequests::from_bytes(&bytes).unwrap();
        match recovered {
            ProviderRequests::PullMessages(req) => {
                assert_eq!(address, req.destination_address);
                assert_eq!(AuthToken(auth_token), req.auth_token);
            }
            _ => panic!("expected to recover pull request!"),
        }
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

        let recovered = RegisterRequest::from_bytes(&bytes).unwrap();
        assert_eq!(address, recovered.destination_address);
    }

    #[test]
    fn it_is_possible_to_recover_it_from_bytes_with_enum_wrapper() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address.clone());
        let bytes = register_request.to_bytes();

        let recovered = ProviderRequests::from_bytes(&bytes).unwrap();
        match recovered {
            ProviderRequests::Register(req) => {
                assert_eq!(address, req.destination_address);
            }
            _ => panic!("expected to recover pull request!"),
        }
    }
}
