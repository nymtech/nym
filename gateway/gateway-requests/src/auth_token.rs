// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub const AUTH_TOKEN_SIZE: usize = 32;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct AuthToken([u8; AUTH_TOKEN_SIZE]);

#[derive(Debug)]
pub enum AuthTokenConversionError {
    InvalidStringError,
    StringOfInvalidLengthError,
}

impl AuthToken {
    pub fn from_bytes(bytes: [u8; AUTH_TOKEN_SIZE]) -> Self {
        AuthToken(bytes)
    }

    pub fn to_bytes(&self) -> [u8; AUTH_TOKEN_SIZE] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, AuthTokenConversionError> {
        let decoded = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(_) => return Err(AuthTokenConversionError::InvalidStringError),
        };

        if decoded.len() != AUTH_TOKEN_SIZE {
            return Err(AuthTokenConversionError::StringOfInvalidLengthError);
        }

        let mut token = [0u8; AUTH_TOKEN_SIZE];
        token.copy_from_slice(&decoded[..]);
        Ok(AuthToken(token))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.0).into_string()
    }
}

impl Into<String> for AuthToken {
    fn into(self) -> String {
        self.to_base58_string()
    }
}

#[cfg(test)]
mod auth_token_conversion {
    use super::*;

    #[test]
    fn it_is_possible_to_recover_it_from_valid_b58_string() {
        let auth_token = AuthToken([42; AUTH_TOKEN_SIZE]);
        let auth_token_string = auth_token.to_base58_string();
        assert_eq!(
            auth_token,
            AuthToken::try_from_base58_string(auth_token_string).unwrap()
        )
    }

    #[test]
    fn it_is_possible_to_recover_it_from_valid_b58_str_ref() {
        let auth_token = AuthToken([42; AUTH_TOKEN_SIZE]);
        let auth_token_string = auth_token.to_base58_string();
        let auth_token_str_ref: &str = &auth_token_string;
        assert_eq!(
            auth_token,
            AuthToken::try_from_base58_string(auth_token_str_ref).unwrap()
        )
    }

    #[test]
    fn it_returns_error_on_b58_with_invalid_characters() {
        let auth_token = AuthToken([42; AUTH_TOKEN_SIZE]);
        let auth_token_string = auth_token.to_base58_string();

        let mut chars = auth_token_string.chars();
        let _consumed_first_char = chars.next().unwrap();

        let invalid_chars_token = "=".to_string() + chars.as_str();
        assert!(AuthToken::try_from_base58_string(invalid_chars_token).is_err())
    }

    #[test]
    fn it_returns_error_on_too_long_b58_string() {
        let auth_token = AuthToken([42; AUTH_TOKEN_SIZE]);
        let mut auth_token_string = auth_token.to_base58_string();
        auth_token_string.push('f');

        assert!(AuthToken::try_from_base58_string(auth_token_string).is_err())
    }

    #[test]
    fn it_returns_error_on_too_short_b58_string() {
        let auth_token = AuthToken([42; AUTH_TOKEN_SIZE]);
        let auth_token_string = auth_token.to_base58_string();

        let mut chars = auth_token_string.chars();
        let _consumed_first_char = chars.next().unwrap();
        let _consumed_second_char = chars.next().unwrap();
        let invalid_chars_token = chars.as_str();

        assert!(AuthToken::try_from_base58_string(invalid_chars_token).is_err())
    }
}
