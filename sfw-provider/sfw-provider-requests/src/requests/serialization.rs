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

use crate::requests::*;
use std::convert::TryFrom;

// TODO: way down the line, mostly for learning purposes, combine this with responses::serialization
// via procedural macros

/// Responsible for taking a request and converting it into bytes that can be sent
/// over the wire, such that a `RequestDeserializer` can recover it.
pub struct RequestSerializer {
    req: ProviderRequest,
}

impl RequestSerializer {
    pub fn new(req: ProviderRequest) -> Self {
        RequestSerializer { req }
    }

    /// Serialized requests in general have the following structure:
    /// 4 byte len (be u32) || 1-byte kind prefix || request-specific data
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

/// Responsible for taking raw bytes extracted from a stream that have been serialized
/// with `RequestSerializer` and eventually return original Request written.
pub struct RequestDeserializer<'a> {
    kind: RequestKind,
    data: &'a [u8],
}

impl<'a> RequestDeserializer<'a> {
    // perform initial parsing and validation
    pub fn new(raw_bytes: &'a [u8]) -> Result<Self, ProviderRequestError> {
        if raw_bytes.len() < 1 + 4 {
            Err(ProviderRequestError::UnmarshalErrorInvalidLength)
        } else {
            let data_len =
                u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]]);

            Self::new_with_len(data_len, &raw_bytes[4..])
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

#[cfg(test)]
mod request_serialization {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use byteorder::{BigEndian, ByteOrder};
    use sphinx::route::DestinationAddressBytes;
    use std::convert::TryInto;

    #[test]
    fn correctly_serializes_pull_request() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let pull_request = PullRequest::new(address, auth_token);

        let raw_request_bytes = pull_request.to_bytes();
        let serializer = RequestSerializer::new(pull_request.clone().into());
        let bytes = serializer.into_bytes();

        // we expect first four bytes to represent length then kind and finally raw data
        let len = BigEndian::read_u32(&bytes);
        let kind: RequestKind = bytes[4].try_into().unwrap();
        let data = &bytes[5..];
        assert_eq!(len as usize, data.len() + 1);
        assert_eq!(data.to_vec(), raw_request_bytes);

        let recovered_request = PullRequest::try_from_bytes(data).unwrap();
        assert_eq!(pull_request, recovered_request);
        assert_eq!(kind, pull_request.get_kind());
    }

    #[test]
    fn correctly_serializes_register_request() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);

        let raw_request_bytes = register_request.to_bytes();
        let serializer = RequestSerializer::new(register_request.clone().into());
        let bytes = serializer.into_bytes();

        // we expect first four bytes to represent length then kind and finally raw data
        let len = BigEndian::read_u32(&bytes);
        let kind: RequestKind = bytes[4].try_into().unwrap();
        let data = &bytes[5..];
        assert_eq!(len as usize, data.len() + 1);
        assert_eq!(data.to_vec(), raw_request_bytes);

        let recovered_request = RegisterRequest::try_from_bytes(data).unwrap();
        assert_eq!(register_request, recovered_request);
        assert_eq!(kind, register_request.get_kind());
    }
}

#[cfg(test)]
mod request_deserialization {
    use super::*;
    use crate::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
    use byteorder::{BigEndian, ByteOrder};
    use sphinx::route::DestinationAddressBytes;

    #[test]
    fn correctly_deserializes_pull_request() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let auth_token = AuthToken::from_bytes([1u8; AUTH_TOKEN_SIZE]);
        let pull_request = PullRequest::new(address, auth_token);

        let raw_request_bytes = pull_request.to_bytes();
        let bytes = RequestSerializer::new(pull_request.clone().into()).into_bytes();

        let deserializer_new = RequestDeserializer::new(&bytes).unwrap();
        assert_eq!(deserializer_new.get_kind(), pull_request.get_kind());
        assert_eq!(deserializer_new.get_data().to_vec(), raw_request_bytes);

        assert_eq!(
            ProviderRequest::Pull(pull_request.clone()),
            deserializer_new.try_to_parse().unwrap()
        );

        // simulate consuming first 4 bytes to read len
        let len = BigEndian::read_u32(&bytes);
        let bytes_without_len = &bytes[4..];
        let deserializer_new_with_len =
            RequestDeserializer::new_with_len(len, bytes_without_len).unwrap();
        assert_eq!(
            deserializer_new_with_len.get_kind(),
            pull_request.get_kind()
        );
        assert_eq!(
            deserializer_new_with_len.get_data().to_vec(),
            raw_request_bytes
        );

        assert_eq!(
            ProviderRequest::Pull(pull_request),
            deserializer_new_with_len.try_to_parse().unwrap()
        );
    }

    #[test]
    fn correctly_deserializes_register_request() {
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);

        let raw_request_bytes = register_request.to_bytes();
        let bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();

        let deserializer_new = RequestDeserializer::new(&bytes).unwrap();
        assert_eq!(deserializer_new.get_kind(), register_request.get_kind());
        assert_eq!(deserializer_new.get_data().to_vec(), raw_request_bytes);

        assert_eq!(
            ProviderRequest::Register(register_request.clone()),
            deserializer_new.try_to_parse().unwrap()
        );

        // simulate consuming first 4 bytes to read len
        let len = BigEndian::read_u32(&bytes);
        let bytes_without_len = &bytes[4..];
        let deserializer_new_with_len =
            RequestDeserializer::new_with_len(len, bytes_without_len).unwrap();
        assert_eq!(
            deserializer_new_with_len.get_kind(),
            register_request.get_kind()
        );
        assert_eq!(
            deserializer_new_with_len.get_data().to_vec(),
            raw_request_bytes
        );

        assert_eq!(
            ProviderRequest::Register(register_request),
            deserializer_new_with_len.try_to_parse().unwrap()
        );
    }

    #[test]
    fn returns_error_on_too_short_messages() {
        // no matter the request, it must be AT LEAST 5 byte long (for length and 'kind')
        let mut len_bytes = 1u32.to_be_bytes().to_vec();
        len_bytes.push(RequestKind::Register as u8); // to have a 'valid' kind

        // bare minimum should return no error
        assert!(RequestDeserializer::new(&len_bytes).is_ok());

        // but shorter should
        assert!(RequestDeserializer::new(&0u32.to_be_bytes().to_vec()).is_err());
    }

    #[test]
    fn returns_error_on_messages_of_contradictory_length() {
        let data = vec![RequestKind::Register as u8, 1, 2, 3];

        // it shouldn't fail if it matches up
        assert!(RequestDeserializer::new_with_len(4, &data).is_ok());

        assert!(RequestDeserializer::new_with_len(3, &data).is_err());
    }

    #[test]
    fn returns_error_on_messages_of_unknown_kind() {
        // perform proper serialization but change 'kind' byte to some invalid value
        let address = DestinationAddressBytes::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]);
        let register_request = RegisterRequest::new(address);
        let mut bytes = RequestSerializer::new(register_request.clone().into()).into_bytes();

        let invalid_kind = 42u8;
        // sanity check to ensure it IS invalid
        assert!(RequestKind::try_from(invalid_kind).is_err());
        bytes[4] = invalid_kind;
        assert!(RequestDeserializer::new(&bytes).is_err());
    }

    #[test]
    fn returns_error_on_parsing_invalid_data() {
        // kind exists, length is correct, but data is unparsable
        // no matter the request, it must be AT LEAST 5 byte long (for length and 'kind')
        let mut len_bytes = 5u32.to_be_bytes().to_vec();
        len_bytes.push(RequestKind::Register as u8); // to have a 'valid' kind
        len_bytes.push(1);
        len_bytes.push(2);
        len_bytes.push(3);
        len_bytes.push(4);

        let deserializer = RequestDeserializer::new(&len_bytes).unwrap();
        assert!(deserializer.try_to_parse().is_err());
    }
}
