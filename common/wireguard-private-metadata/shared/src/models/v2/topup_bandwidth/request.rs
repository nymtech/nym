// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::BandwidthCredential;
use serde::{Deserialize, Serialize};

use crate::impl_default_bincode_request_query_conversions;

use super::super::{QueryType, VersionedRequest};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerTopUpRequest {
    /// Ecash credential
    pub credential: BandwidthCredential,
}

// Implements:
// - TryFrom<&VersionedRequest> for InnerTopUpRequest
// - TryFrom<VersionedRequest> for InnerTopUpRequest
// - TryFrom<&InnerTopUpRequest> for VersionedRequest
// - TryFrom<InnerTopUpRequest> for VersionedRequest
// - TryFrom<&Request> for InnerTopUpRequest
// - TryFrom<Request> for InnerTopUpRequest
// - TryFrom<&InnerTopUpRequest> for Request
// - TryFrom<InnerTopUpRequest> for Request
impl_default_bincode_request_query_conversions!(
    VersionedRequest,
    InnerTopUpRequest,
    QueryType::TopUpBandwidth,
    QueryType::TopUpBandwidth
);

#[cfg(test)]
mod tests {
    use crate::models::tests::CREDENTIAL_BYTES;
    use nym_credentials_interface::CredentialSpendingData;

    use super::*;

    #[test]
    fn serde() {
        let req = InnerTopUpRequest {
            credential: BandwidthCredential::from(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
            ),
        };
        let ser = VersionedRequest::try_from(req.clone()).unwrap();
        assert_eq!(QueryType::TopUpBandwidth, ser.query_type);
        let de = InnerTopUpRequest::try_from(ser).unwrap();
        assert_eq!(req, de);
    }

    #[test]
    fn invalid_content() {
        let future_req = VersionedRequest {
            query_type: QueryType::TopUpBandwidth,
            inner: vec![],
        };
        assert!(InnerTopUpRequest::try_from(future_req).is_err());
    }
}
