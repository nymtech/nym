// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::impl_default_bincode_request_query_conversions;

use super::super::{QueryType, VersionedRequest};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UpgradeModeCheckRequestType {
    /// Attempt to request upgrade mode recheck via the JWT issued as the result of
    /// global attestation.json being published
    UpgradeModeJwt { token: String },
}

// each versioned variant should always be a subset of the latest one defined in the interface
// so a From trait should always be implementable (as opposed to having to do TryFrom)
// (but this is not applicable in this instance as this IS the latest (05.11.25)
// impl From<UpgradeModeCheckRequestType> for crate::models::interface::UpgradeModeCheckRequestType {
//     fn from(typ: UpgradeModeCheckRequestType) -> Self {
//         match typ {
//             UpgradeModeCheckRequestType::UpgradeModeJwt { token } => {
//                 crate::models::interface::UpgradeModeCheckRequestType::UpgradeModeJwt { token }
//             }
//         }
//     }
// }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerUpgradeModeCheckRequest {
    pub request_type: UpgradeModeCheckRequestType,
}

// Implements:
// - TryFrom<&VersionedRequest> for InnerUpgradeModeCheckRequest
// - TryFrom<VersionedRequest> for InnerUpgradeModeCheckRequest
// - TryFrom<&InnerUpgradeModeCheckRequest> for VersionedRequest
// - TryFrom<InnerUpgradeModeCheckRequest> for VersionedRequest
// - TryFrom<&Request> for InnerUpgradeModeCheckRequest
// - TryFrom<Request> for InnerUpgradeModeCheckRequest
// - TryFrom<&InnerUpgradeModeCheckRequest> for Request
// - TryFrom<InnerUpgradeModeCheckRequest> for Request
impl_default_bincode_request_query_conversions!(
    VersionedRequest,
    InnerUpgradeModeCheckRequest,
    QueryType::UpgradeModeCheck,
    QueryType::UpgradeModeCheck
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let req = InnerUpgradeModeCheckRequest {
            request_type: UpgradeModeCheckRequestType::UpgradeModeJwt {
                token: "dummy.jwt.token".to_string(),
            },
        };
        let ser = VersionedRequest::try_from(req.clone()).unwrap();
        assert_eq!(QueryType::UpgradeModeCheck, ser.query_type);
        let de = InnerUpgradeModeCheckRequest::try_from(ser).unwrap();
        assert_eq!(req, de);
    }

    #[test]
    fn invalid_content() {
        let future_req = VersionedRequest {
            query_type: QueryType::UpgradeModeCheck,
            inner: vec![],
        };
        assert!(InnerUpgradeModeCheckRequest::try_from(future_req).is_err());
    }
}
