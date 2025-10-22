// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::impl_default_bincode_request_query_conversions;

use super::super::{QueryType, VersionedRequest};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InnerAvailableBandwidthRequest {}

// Implements:
// - TryFrom<&VersionedRequest> for InnerTopUpRequest
// - TryFrom<VersionedRequest> for InnerTopUpRequest
// - TryFrom<&InnerTopUpRequest> for VersionedRequest
// - TryFrom<InnerTopUpRequest> for VersionedRequest
// - TryFrom<&Request> for InnerAvailableBandwidthRequest
// - TryFrom<Request> for InnerAvailableBandwidthRequest
// - TryFrom<&InnerTopUpRequest> for Request
// - TryFrom<InnerTopUpRequest> for Request
impl_default_bincode_request_query_conversions!(
    VersionedRequest,
    InnerAvailableBandwidthRequest,
    QueryType::AvailableBandwidth,
    QueryType::AvailableBandwidth
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let req = InnerAvailableBandwidthRequest {};
        let ser = VersionedRequest::try_from(req).unwrap();
        assert_eq!(QueryType::AvailableBandwidth, ser.query_type);
        let de = InnerAvailableBandwidthRequest::try_from(ser).unwrap();
        assert_eq!(req, de);
    }

    #[test]
    fn empty_content() {
        let future_req = VersionedRequest {
            query_type: QueryType::AvailableBandwidth,
            inner: vec![],
        };
        assert!(InnerAvailableBandwidthRequest::try_from(future_req).is_ok());
    }
}
