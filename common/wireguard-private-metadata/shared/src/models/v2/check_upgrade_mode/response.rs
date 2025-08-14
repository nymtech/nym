// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::impl_default_bincode_response_query_conversions;

use super::super::{QueryType, VersionedResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerUpgradeModeCheckResponse {
    pub upgrade_mode: bool,
}

// Implements:
// - TryFrom<&VersionedResponse> for InnerUpgradeModeCheckResponse
// - TryFrom<VersionedResponse> for InnerUpgradeModeCheckResponse
// - TryFrom<&InnerUpgradeModeCheckResponse> for VersionedResponse
// - TryFrom<InnerUpgradeModeCheckResponse> for VersionedResponse
// - TryFrom<&Response> for InnerUpgradeModeCheckResponse
// - TryFrom<Response> for InnerUpgradeModeCheckResponse
// - TryFrom<&InnerUpgradeModeCheckResponse> for Response
// - TryFrom<InnerUpgradeModeCheckResponse> for Response
impl_default_bincode_response_query_conversions!(
    VersionedResponse,
    InnerUpgradeModeCheckResponse,
    QueryType::UpgradeModeCheck,
    QueryType::UpgradeModeCheck
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let resp = InnerUpgradeModeCheckResponse { upgrade_mode: true };
        let ser = VersionedResponse::try_from(resp.clone()).unwrap();
        assert_eq!(QueryType::UpgradeModeCheck, ser.query_type);
        let de = InnerUpgradeModeCheckResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }

    #[test]
    fn invalid_content() {
        let future_resp = VersionedResponse {
            query_type: QueryType::UpgradeModeCheck,
            inner: vec![],
        };
        assert!(InnerUpgradeModeCheckResponse::try_from(future_resp).is_err());
    }
}
