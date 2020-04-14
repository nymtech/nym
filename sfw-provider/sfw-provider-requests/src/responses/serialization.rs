use crate::responses::{
    FailureResponse, ProviderResponse, ProviderResponseError, PullResponse, RegisterResponse,
    ResponseKind,
};
use std::convert::TryFrom;

// TODO: way down the line, mostly for learning purposes, combine this with requests::serialization
// via procedural macros

pub struct ResponseDeserializer<'a> {
    kind: ResponseKind,
    data: &'a [u8],
}

impl<'a> ResponseDeserializer<'a> {
    // perform initial parsing
    pub fn new(raw_bytes: &'a [u8]) -> Result<Self, ProviderResponseError> {
        if raw_bytes.len() < 1 + 4 {
            Err(ProviderResponseError::UnmarshalErrorInvalidLength)
        } else {
            let data_len =
                u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]]);
            let kind = ResponseKind::try_from(raw_bytes[4])?;
            let data = &raw_bytes[4..];

            if data.len() != data_len as usize {
                Err(ProviderResponseError::UnmarshalErrorInvalidLength)
            } else {
                Ok(ResponseDeserializer { kind, data })
            }
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
