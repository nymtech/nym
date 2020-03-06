pub mod requests;
pub mod responses;

pub const DUMMY_MESSAGE_CONTENT: &[u8] =
    b"[DUMMY MESSAGE] Wanting something does not give you the right to have it.";

// To be renamed to 'AuthToken' once it is safe to replace it
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AuthToken([u8; 32]);

#[derive(Debug)]
pub enum AuthTokenConversionError {
    InvalidStringError,
    StringOfInvalidLengthError,
}

impl AuthToken {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        AuthToken(bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
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

        if decoded.len() != 32 {
            return Err(AuthTokenConversionError::StringOfInvalidLengthError);
        }

        let mut token = [0u8; 32];
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
        let auth_token = AuthToken([42; 32]);
        let auth_token_string = auth_token.to_base58_string();
        assert_eq!(
            auth_token,
            AuthToken::try_from_base58_string(auth_token_string).unwrap()
        )
    }

    #[test]
    fn it_is_possible_to_recover_it_from_valid_b58_str_ref() {
        let auth_token = AuthToken([42; 32]);
        let auth_token_string = auth_token.to_base58_string();
        let auth_token_str_ref: &str = &auth_token_string;
        assert_eq!(
            auth_token,
            AuthToken::try_from_base58_string(auth_token_str_ref).unwrap()
        )
    }

    #[test]
    fn it_returns_error_on_b58_with_invalid_characters() {
        let auth_token = AuthToken([42; 32]);
        let auth_token_string = auth_token.to_base58_string();

        let mut chars = auth_token_string.chars();
        let _consumed_first_char = chars.next().unwrap();

        let invalid_chars_token = "=".to_string() + chars.as_str();
        assert!(AuthToken::try_from_base58_string(invalid_chars_token).is_err())
    }

    #[test]
    fn it_returns_error_on_too_long_b58_string() {
        let auth_token = AuthToken([42; 32]);
        let mut auth_token_string = auth_token.to_base58_string();
        auth_token_string.push('f');

        assert!(AuthToken::try_from_base58_string(auth_token_string).is_err())
    }

    #[test]
    fn it_returns_error_on_too_short_b58_string() {
        let auth_token = AuthToken([42; 32]);
        let auth_token_string = auth_token.to_base58_string();

        let mut chars = auth_token_string.chars();
        let _consumed_first_char = chars.next().unwrap();
        let _consumed_second_char = chars.next().unwrap();
        let invalid_chars_token = chars.as_str();

        assert!(AuthToken::try_from_base58_string(invalid_chars_token).is_err())
    }
}
