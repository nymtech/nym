use crate::AuthToken;
use std::convert::TryInto;

#[derive(Debug)]
pub enum ProviderResponseError {
    MarshalError,
    UnmarshalError,
    UnmarshalErrorInvalidLength,
}

pub trait ProviderResponse
where
    Self: Sized,
{
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError>;
}

#[derive(Debug)]
pub struct PullResponse {
    pub messages: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub struct RegisterResponse {
    pub auth_token: AuthToken,
}

pub struct ErrorResponse {
    pub message: String,
}

impl PullResponse {
    pub fn new(messages: Vec<Vec<u8>>) -> Self {
        PullResponse { messages }
    }
}

impl RegisterResponse {
    pub fn new(auth_token: AuthToken) -> Self {
        RegisterResponse { auth_token }
    }
}

impl ErrorResponse {
    pub fn new<S: Into<String>>(message: S) -> Self {
        ErrorResponse {
            message: message.into(),
        }
    }
}

// TODO: This should go into some kind of utils module/crate
fn read_be_u16(input: &mut &[u8]) -> u16 {
    let (int_bytes, rest) = input.split_at(std::mem::size_of::<u16>());
    *input = rest;
    u16::from_be_bytes(int_bytes.try_into().unwrap())
}

// TODO: currently this allows for maximum 64kB payload -
// if we go over that in sphinx we need to update this code.
impl ProviderResponse for PullResponse {
    // num_msgs || len1 || len2 || ... || msg1 || msg2 || ...
    fn to_bytes(&self) -> Vec<u8> {
        let num_msgs = self.messages.len() as u16;
        let msgs_lens: Vec<u16> = self.messages.iter().map(|msg| msg.len() as u16).collect();

        num_msgs
            .to_be_bytes()
            .to_vec()
            .into_iter()
            .chain(
                msgs_lens
                    .into_iter()
                    .flat_map(|len| len.to_be_bytes().to_vec().into_iter()),
            )
            .chain(self.messages.iter().flat_map(|msg| msg.clone().into_iter()))
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError> {
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

impl ProviderResponse for RegisterResponse {
    fn to_bytes(&self) -> Vec<u8> {
        self.auth_token.0.to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError> {
        match bytes.len() {
            32 => {
                let mut auth_token = [0u8; 32];
                auth_token.copy_from_slice(&bytes[..32]);
                Ok(RegisterResponse {
                    auth_token: AuthToken(auth_token),
                })
            }
            _ => Err(ProviderResponseError::UnmarshalErrorInvalidLength),
        }
    }
}

impl ProviderResponse for ErrorResponse {
    fn to_bytes(&self) -> Vec<u8> {
        self.message.clone().into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderResponseError> {
        match String::from_utf8(bytes.to_vec()) {
            Err(_) => Err(ProviderResponseError::UnmarshalError),
            Ok(message) => Ok(ErrorResponse { message }),
        }
    }
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

        let recovered = PullResponse::from_bytes(&bytes).unwrap();
        assert_eq!(msgs, recovered.messages);
    }
}
