// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::impl_default_bincode_response_query_conversions;

use super::super::{QueryType, VersionedResponse};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InnerAvailableBandwidthResponse {}

// Implements:
// - TryFrom<&VersionedResponse> for InnerAvailableBandwidthResponse
// - TryFrom<VersionedResponse> for InnerAvailableBandwidthResponse
// - TryFrom<&InnerAvailableBandwidthResponse> for VersionedResponse
// - TryFrom<InnerAvailableBandwidthResponse> for VersionedResponse
// - TryFrom<&Response> for InnerAvailableBandwidthResponse
// - TryFrom<Response> for InnerAvailableBandwidthResponse
// - TryFrom<&InnerAvailableBandwidthResponse> for Response
// - TryFrom<InnerAvailableBandwidthResponse> for Response
impl_default_bincode_response_query_conversions!(
    VersionedResponse,
    InnerAvailableBandwidthResponse,
    QueryType::AvailableBandwidth,
    QueryType::AvailableBandwidth
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let resp = InnerAvailableBandwidthResponse {};
        let ser = VersionedResponse::try_from(resp).unwrap();
        assert_eq!(QueryType::AvailableBandwidth, ser.query_type);
        let de = InnerAvailableBandwidthResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }

    #[test]
    fn empty_content() {
        let future_resp = VersionedResponse {
            query_type: QueryType::AvailableBandwidth,
            inner: vec![],
        };
        assert!(InnerAvailableBandwidthResponse::try_from(future_resp).is_ok());
    }
}
