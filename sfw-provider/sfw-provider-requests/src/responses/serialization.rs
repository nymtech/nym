use crate::responses::*;
use std::convert::TryFrom;

// TODO: way down the line, mostly for learning purposes, combine this with requests::serialization
// via procedural macros

/// Responsible for taking a response and converting it into bytes that can be sent
/// over the wire, such that a `ResponseDeserializer` can recover it.
pub struct ResponseSerializer {
    res: ProviderResponse,
}

impl ResponseSerializer {
    pub fn new(res: ProviderResponse) -> Self {
        ResponseSerializer { res }
    }

    /// Serialized responses in general have the following structure:
    /// 4 byte len (be u32) || 1-byte kind prefix || response-specific data
    pub fn into_bytes(self) -> Vec<u8> {
        let (kind, res_bytes) = match self.res {
            // again, perhaps some extra macros/generics here?
            ProviderResponse::Failure(res) => (res.get_kind(), res.to_bytes()),
            ProviderResponse::Pull(res) => (res.get_kind(), res.to_bytes()),
            ProviderResponse::Register(res) => (res.get_kind(), res.to_bytes()),
        };
        let res_len = res_bytes.len() as u32 + 1; // 1 is to accommodate for 'kind'
        let res_len_bytes = res_len.to_be_bytes();
        res_len_bytes
            .iter()
            .cloned()
            .chain(std::iter::once(kind as u8))
            .chain(res_bytes.into_iter())
            .collect()
    }
}

/// Responsible for taking raw bytes extracted from a stream that have been serialized
/// with `ResponseSerializer` and eventually return original Response written.
pub struct ResponseDeserializer<'a> {
    kind: ResponseKind,
    data: &'a [u8],
}

impl<'a> ResponseDeserializer<'a> {
    // perform initial parsing and validation
    pub fn new(raw_bytes: &'a [u8]) -> Result<Self, ProviderResponseError> {
        if raw_bytes.len() < 1 + 4 {
            Err(ProviderResponseError::UnmarshalErrorInvalidLength)
        } else {
            let data_len =
                u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]]);

            Self::new_with_len(data_len, &raw_bytes[4..])
        }
    }

    pub fn new_with_len(len: u32, raw_bytes: &'a [u8]) -> Result<Self, ProviderResponseError> {
        if raw_bytes.len() != len as usize {
            Err(ProviderResponseError::UnmarshalErrorInvalidLength)
        } else {
            let kind = ResponseKind::try_from(raw_bytes[0])?;
            let data = &raw_bytes[1..];
            Ok(ResponseDeserializer { kind, data })
        }
    }

    pub fn get_kind(&self) -> ResponseKind {
        self.kind
    }

    pub fn get_data(&self) -> &'a [u8] {
        self.data
    }

    pub fn try_to_parse(self) -> Result<ProviderResponse, ProviderResponseError> {
        match self.get_kind() {
            ResponseKind::Failure => Ok(ProviderResponse::Failure(
                FailureResponse::try_from_bytes(self.data)?,
            )),
            ResponseKind::Pull => Ok(ProviderResponse::Pull(PullResponse::try_from_bytes(
                self.data,
            )?)),
            ResponseKind::Register => Ok(ProviderResponse::Register(
                RegisterResponse::try_from_bytes(self.data)?,
            )),
        }
    }
}

#[cfg(test)]
mod response_serialization {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use byteorder::{BigEndian, ByteOrder};
    use std::convert::TryInto;

    #[test]
    fn correctly_serializes_pull_response() {
        let msg1 = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ];
        let msg2 = vec![1, 2, 3, 4, 5, 6, 7];
        let pull_response = PullResponse::new(vec![msg1, msg2]);

        let raw_response_bytes = pull_response.to_bytes();
        let serializer = ResponseSerializer::new(pull_response.clone().into());
        let bytes = serializer.into_bytes();

        // we expect first four bytes to represent length then kind and finally raw data
        let len = BigEndian::read_u32(&bytes);
        let kind: ResponseKind = bytes[4].try_into().unwrap();
        let data = &bytes[5..];
        assert_eq!(len as usize, data.len() + 1);
        assert_eq!(data.to_vec(), raw_response_bytes);

        let recovered_response = PullResponse::try_from_bytes(data).unwrap();
        assert_eq!(pull_response, recovered_response);
        assert_eq!(kind, pull_response.get_kind());
    }

    #[test]
    fn correctly_serializes_register_response() {
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);

        let raw_response_bytes = register_response.to_bytes();
        let serializer = ResponseSerializer::new(register_response.clone().into());
        let bytes = serializer.into_bytes();

        // we expect first four bytes to represent length then kind and finally raw data
        let len = BigEndian::read_u32(&bytes);
        let kind: ResponseKind = bytes[4].try_into().unwrap();
        let data = &bytes[5..];
        assert_eq!(len as usize, data.len() + 1);
        assert_eq!(data.to_vec(), raw_response_bytes);

        let recovered_response = RegisterResponse::try_from_bytes(data).unwrap();
        assert_eq!(register_response, recovered_response);
        assert_eq!(kind, register_response.get_kind());
    }
}

#[cfg(test)]
mod response_deserialization {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use byteorder::{BigEndian, ByteOrder};

    #[test]
    fn correctly_deserializes_pull_response() {
        let msg1 = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ];
        let msg2 = vec![1, 2, 3, 4, 5, 6, 7];
        let pull_response = PullResponse::new(vec![msg1, msg2]);

        let raw_response_bytes = pull_response.to_bytes();
        let bytes = ResponseSerializer::new(pull_response.clone().into()).into_bytes();

        let deserializer_new = ResponseDeserializer::new(&bytes).unwrap();
        assert_eq!(deserializer_new.get_kind(), pull_response.get_kind());
        assert_eq!(deserializer_new.get_data().to_vec(), raw_response_bytes);

        assert_eq!(
            ProviderResponse::Pull(pull_response.clone()),
            deserializer_new.try_to_parse().unwrap()
        );

        // simulate consuming first 4 bytes to read len
        let len = BigEndian::read_u32(&bytes);
        let bytes_without_len = &bytes[4..];
        let deserializer_new_with_len =
            ResponseDeserializer::new_with_len(len, bytes_without_len).unwrap();
        assert_eq!(
            deserializer_new_with_len.get_kind(),
            pull_response.get_kind()
        );
        assert_eq!(
            deserializer_new_with_len.get_data().to_vec(),
            raw_response_bytes
        );

        assert_eq!(
            ProviderResponse::Pull(pull_response),
            deserializer_new_with_len.try_to_parse().unwrap()
        );
    }

    #[test]
    fn correctly_deserializes_register_response() {
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);

        let raw_response_bytes = register_response.to_bytes();
        let bytes = ResponseSerializer::new(register_response.clone().into()).into_bytes();

        let deserializer_new = ResponseDeserializer::new(&bytes).unwrap();
        assert_eq!(deserializer_new.get_kind(), register_response.get_kind());
        assert_eq!(deserializer_new.get_data().to_vec(), raw_response_bytes);

        assert_eq!(
            ProviderResponse::Register(register_response.clone()),
            deserializer_new.try_to_parse().unwrap()
        );

        // simulate consuming first 4 bytes to read len
        let len = BigEndian::read_u32(&bytes);
        let bytes_without_len = &bytes[4..];
        let deserializer_new_with_len =
            ResponseDeserializer::new_with_len(len, bytes_without_len).unwrap();
        assert_eq!(
            deserializer_new_with_len.get_kind(),
            register_response.get_kind()
        );
        assert_eq!(
            deserializer_new_with_len.get_data().to_vec(),
            raw_response_bytes
        );

        assert_eq!(
            ProviderResponse::Register(register_response),
            deserializer_new_with_len.try_to_parse().unwrap()
        );
    }

    #[test]
    fn returns_error_on_too_short_messages() {
        // no matter the response, it must be AT LEAST 5 byte long (for length and 'kind')
        let mut len_bytes = 1u32.to_be_bytes().to_vec();
        len_bytes.push(ResponseKind::Register as u8); // to have a 'valid' kind

        // bare minimum should return no error
        assert!(ResponseDeserializer::new(&len_bytes).is_ok());

        // but shorter should
        assert!(ResponseDeserializer::new(&0u32.to_be_bytes().to_vec()).is_err());
    }

    #[test]
    fn returns_error_on_messages_of_contradictory_length() {
        let data = vec![ResponseKind::Register as u8, 1, 2, 3];

        // it shouldn't fail if it matches up
        assert!(ResponseDeserializer::new_with_len(4, &data).is_ok());

        assert!(ResponseDeserializer::new_with_len(3, &data).is_err());
    }

    #[test]
    fn returns_error_on_messages_of_unknown_kind() {
        // perform proper serialization but change 'kind' byte to some invalid value
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let register_response = RegisterResponse::new(auth_token);

        let mut bytes = ResponseSerializer::new(register_response.clone().into()).into_bytes();

        let invalid_kind = 42u8;
        // sanity check to ensure it IS invalid
        assert!(ResponseKind::try_from(invalid_kind).is_err());
        bytes[4] = invalid_kind;
        assert!(ResponseDeserializer::new(&bytes).is_err());
    }

    #[test]
    fn returns_error_on_parsing_invalid_data() {
        // kind exists, length is correct, but data is unparsable
        // no matter the response, it must be AT LEAST 5 byte long (for length and 'kind')
        let mut len_bytes = 5u32.to_be_bytes().to_vec();
        len_bytes.push(ResponseKind::Register as u8); // to have a 'valid' kind
        len_bytes.push(1);
        len_bytes.push(2);
        len_bytes.push(3);
        len_bytes.push(4);

        let deserializer = ResponseDeserializer::new(&len_bytes).unwrap();
        assert!(deserializer.try_to_parse().is_err());
    }
}
