// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::impl_default_bincode_response_query_conversions;

use super::super::{QueryType, VersionedResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerTopUpResponse {}

// Implements:
// - TryFrom<&VersionedResponse> for InnerTopUpResponse
// - TryFrom<VersionedResponse> for InnerTopUpResponse
// - TryFrom<&InnerTopUpResponse> for VersionedResponse
// - TryFrom<InnerTopUpResponse> for VersionedResponse
// - TryFrom<&Response> for InnerTopUpResponse
// - TryFrom<Response> for InnerTopUpResponse
// - TryFrom<&InnerTopUpResponse> for Response
// - TryFrom<InnerTopUpResponse> for Response
impl_default_bincode_response_query_conversions!(
    VersionedResponse,
    InnerTopUpResponse,
    QueryType::TopUpBandwidth,
    QueryType::TopUpBandwidth
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let resp = InnerTopUpResponse {};
        let ser = VersionedResponse::try_from(resp.clone()).unwrap();
        assert_eq!(QueryType::TopUpBandwidth, ser.query_type);
        let de = InnerTopUpResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }

    #[test]
    fn empty_content() {
        let future_resp = VersionedResponse {
            query_type: QueryType::TopUpBandwidth,
            inner: vec![],
        };
        assert!(InnerTopUpResponse::try_from(future_resp).is_ok());
    }
}
