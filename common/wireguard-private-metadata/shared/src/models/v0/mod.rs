// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    impl_default_bincode_request_conversions, impl_default_bincode_response_conversions,
    models::Version,
};

pub(crate) mod available_bandwidth;
pub mod interface;
pub(crate) mod topup_bandwidth;

pub const VERSION: Version = Version::V0;

pub use available_bandwidth::{
    request::InnerAvailableBandwidthRequest as AvailableBandwidthRequest,
    response::InnerAvailableBandwidthResponse as AvailableBandwidthResponse,
};
pub use topup_bandwidth::{
    request::InnerTopUpRequest as TopUpRequest, response::InnerTopUpResponse as TopUpResponse,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum QueryType {
    AvailableBandwidth,
    TopUpBandwidth,
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct VersionedRequest {
    query_type: QueryType,
    inner: Vec<u8>,
}
// Implements:
// - TryFrom<&VersionedRequest> for Request
// - TryFrom<VersionedRequest> for Request
// - TryFrom<&Request> for VersionedRequest
// - TryFrom<Request> for VersionedRequest
impl_default_bincode_request_conversions!(VersionedRequest, VERSION);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct VersionedResponse {
    query_type: QueryType,
    inner: Vec<u8>,
}
// Implements:
// - TryFrom<&VersionedResponse> for Response
// - TryFrom<VersionedResponse> for Response
// - TryFrom<&Response> for VersionedResponse
// - TryFrom<Response> for VersionedResponse
impl_default_bincode_response_conversions!(VersionedResponse, VERSION);

#[cfg(test)]
mod tests {
    use self::{
        available_bandwidth::{
            request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
        },
        topup_bandwidth::{request::InnerTopUpRequest, response::InnerTopUpResponse},
    };
    use super::*;
    use crate::make_bincode_serializer;
    use crate::{Request, Response};
    use bincode::Options;

    #[test]
    fn serde_request_av_bw() {
        let req = VersionedRequest {
            query_type: QueryType::AvailableBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerAvailableBandwidthRequest {})
                .unwrap(),
        };

        let ser = Request::try_from(req.clone()).unwrap();
        assert_eq!(VERSION, ser.version);
        let de = VersionedRequest::try_from(ser).unwrap();
        assert_eq!(req, de);
    }

    #[test]
    fn serde_response_av_bw() {
        let resp = VersionedResponse {
            query_type: QueryType::AvailableBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerAvailableBandwidthResponse {})
                .unwrap(),
        };

        let ser = Response::try_from(resp.clone()).unwrap();
        assert_eq!(VERSION, ser.version);
        let de = VersionedResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }

    #[test]
    fn serde_request_topup() {
        let req = VersionedRequest {
            query_type: QueryType::TopUpBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerTopUpRequest {})
                .unwrap(),
        };

        let ser = Request::try_from(req.clone()).unwrap();
        assert_eq!(VERSION, ser.version);
        let de = VersionedRequest::try_from(ser).unwrap();
        assert_eq!(req, de);
    }

    #[test]
    fn serde_response_topup() {
        let resp = VersionedResponse {
            query_type: QueryType::TopUpBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerTopUpResponse {})
                .unwrap(),
        };

        let ser = Response::try_from(resp.clone()).unwrap();
        assert_eq!(VERSION, ser.version);
        let de = VersionedResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }
}
