use crate::requests::{
    ProviderRequest, ProviderRequestError, PullRequest, RegisterRequest, RequestKind,
};
use std::convert::TryFrom;

// TODO: way down the line, mostly for learning purposes, combine this with responses::serialization
// via procedural macros

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
