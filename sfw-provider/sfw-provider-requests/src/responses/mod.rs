use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
use std::convert::{TryFrom, TryInto};
use std::io;
use std::io::Error;

pub mod async_io;
pub mod serialization;

#[derive(Debug)]
pub enum ProviderResponseError {
    MarshalError,
    UnmarshalError,
    UnmarshalErrorInvalidKind,
    UnmarshalErrorInvalidLength,
    TooShortResponseError,
    TooLongResponseError,
    IOError(io::Error),
    RemoteConnectionClosed,
}

impl From<io::Error> for ProviderResponseError {
    fn from(e: Error) -> Self {
        ProviderResponseError::IOError(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ResponseKind {
    Failure = 0, // perhaps Error would have been a better name, but it'd clash with Self::TryFrom<u8>::Error
    Pull = 1,
    Register = 2,
}

impl TryFrom<u8> for ResponseKind {
    type Error = ProviderResponseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (ResponseKind::Failure as u8) => Ok(Self::Register),
            _ if value == (ResponseKind::Pull as u8) => Ok(Self::Pull),
            _ if value == (ResponseKind::Register as u8) => Ok(Self::Register),
            _ => Err(Self::Error::UnmarshalErrorInvalidKind),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderResponse {
    Failure(FailureResponse),
    Pull(PullResponse),
    Register(RegisterResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PullResponse {
    messages: Vec<Vec<u8>>,
}

impl PullResponse {
    pub fn new(messages: Vec<Vec<u8>>) -> Self {
        PullResponse { messages }
    }

    pub fn extract_messages(self) -> Vec<Vec<u8>> {
        self.messages
    }

    pub fn get_kind(&self) -> ResponseKind {
        ResponseKind::Pull
    }

    // TODO: currently this allows for maximum 64kB payload (max value of u16) -
    // if we go over that in sphinx we need to update this code.
    // num_msgs || len1 || len2 || ... || msg1 || msg2 || ...
    pub fn to_bytes(&self) -> Vec<u8> {
        let num_msgs = self.messages.len() as u16;
        let msgs_lens: Vec<u16> = self.messages.iter().map(|msg| msg.len() as u16).collect();

        num_msgs
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(
                msgs_lens
                    .into_iter()
                    .flat_map(|len| len.to_be_bytes().to_vec().into_iter()),
            )
            .chain(self.messages.iter().flat_map(|msg| msg.clone().into_iter()))
            .collect()
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError> {
        // can we read number of messages?
        if bytes.len() < 2 {
            return Err(ProviderResponseError::UnmarshalErrorInvalidLength);
        }

        let mut bytes_copy = bytes;
        let num_msgs = read_be_u16(&mut bytes_copy);

        // can we read all lengths of messages?
        if bytes_copy.len() < (num_msgs * 2) as usize {
            return Err(ProviderResponseError::UnmarshalErrorInvalidLength);
        }

        let msgs_lens: Vec<_> = (0..num_msgs)
            .map(|_| read_be_u16(&mut bytes_copy))
            .collect();

        let required_remaining_len = msgs_lens
            .iter()
            .fold(0usize, |acc, &len| acc + (len as usize));

        // can we read messages themselves?
        if bytes_copy.len() != required_remaining_len {
            return Err(ProviderResponseError::UnmarshalErrorInvalidLength);
        }

        let msgs = msgs_lens
            .iter()
            .scan(0usize, |i, &len| {
                let j = *i + (len as usize);
                let msg = bytes_copy[*i..j].to_vec();
                *i = j;
                Some(msg)
            })
            .collect();

        Ok(PullResponse { messages: msgs })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterResponse {
    auth_token: AuthToken,
}

impl RegisterResponse {
    pub fn new(auth_token: AuthToken) -> Self {
        RegisterResponse { auth_token }
    }

    pub fn get_token(&self) -> AuthToken {
        self.auth_token
    }

    pub fn get_kind(&self) -> ResponseKind {
        ResponseKind::Register
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.auth_token.to_bytes().to_vec()
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError> {
        if bytes.len() != AUTH_TOKEN_SIZE {
            return Err(ProviderResponseError::UnmarshalErrorInvalidLength);
        }

        let mut auth_token = [0u8; AUTH_TOKEN_SIZE];
        auth_token.copy_from_slice(&bytes[..AUTH_TOKEN_SIZE]);
        Ok(RegisterResponse {
            auth_token: AuthToken::from_bytes(auth_token),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FailureResponse {
    message: String,
}

impl FailureResponse {
    pub fn new<S: Into<String>>(message: S) -> Self {
        FailureResponse {
            message: message.into(),
        }
    }

    pub fn get_message(&self) -> &str {
        &self.message
    }

    pub fn get_kind(&self) -> ResponseKind {
        ResponseKind::Failure
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.message.clone().into_bytes()
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError> {
        match String::from_utf8(bytes.to_vec()) {
            Err(_) => Err(ProviderResponseError::UnmarshalError),
            Ok(message) => Ok(FailureResponse { message }),
        }
    }
}

fn read_be_u16(input: &mut &[u8]) -> u16 {
    let (int_bytes, rest) = input.split_at(std::mem::size_of::<u16>());
    *input = rest;
    u16::from_be_bytes(int_bytes.try_into().unwrap())
}

#[cfg(test)]
mod creating_pull_response {
    use super::*;

    #[test]
    fn it_is_possible_to_recover_it_from_bytes() {
        let msg1 = vec![1, 2, 3, 4, 5];
        let msg2 = vec![];
        let msg3 = vec![
            1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4,
            5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3,
            4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2,
            3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1,
            2, 3, 4, 5, 1, 2, 3, 4, 5,
        ];
        let msg4 = vec![1, 2, 3, 4, 5, 6, 7];

        let msgs = vec![msg1, msg2, msg3, msg4];
        let pull_response = PullResponse::new(msgs.clone());
        let bytes = pull_response.to_bytes();

        let recovered = PullResponse::try_from_bytes(&bytes).unwrap();
        assert_eq!(msgs, recovered.messages);
    }
}
