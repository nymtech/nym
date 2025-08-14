// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use bincode::Options;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::error::Error;
use crate::{
    make_bincode_serializer,
    models::{Request, Response, Version},
};

pub use available_bandwidth::{
    request::InnerAvailableBandwidthRequest as AvailableBandwidthRequest,
    response::InnerAvailableBandwidthResponse as AvailableBandwidthResponse,
};
pub use topup_bandwidth::{
    request::InnerTopUpRequest as TopUpRequest, response::InnerTopUpResponse as TopUpResponse,
};

pub(crate) mod available_bandwidth;
pub(crate) mod interface;
pub(crate) mod topup_bandwidth;

pub const VERSION: Version = Version::V1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum QueryType {
    AvailableBandwidth,
    TopupBandwidth,
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

impl TryFrom<VersionedRequest> for Request {
    type Error = Error;

    fn try_from(value: VersionedRequest) -> Result<Self, Self::Error> {
        Ok(Request {
            version: VERSION,
            inner: make_bincode_serializer().serialize(&value)?,
        })
    }
}

impl TryFrom<Request> for VersionedRequest {
    type Error = Error;

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        if value.version != VERSION {
            return Err(Error::InvalidVersion {
                source_version: value.version,
                target_version: VERSION,
            });
        }
        Ok(make_bincode_serializer().deserialize(&value.inner)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct VersionedResponse {
    query_type: QueryType,
    inner: Vec<u8>,
}

impl TryFrom<VersionedResponse> for Response {
    type Error = Error;

    fn try_from(value: VersionedResponse) -> Result<Self, Self::Error> {
        Ok(Response {
            version: VERSION,
            inner: make_bincode_serializer().serialize(&value)?,
        })
    }
}

impl TryFrom<Response> for VersionedResponse {
    type Error = Error;

    fn try_from(value: Response) -> Result<Self, Self::Error> {
        if value.version != VERSION {
            return Err(Error::InvalidVersion {
                source_version: value.version,
                target_version: VERSION,
            });
        }
        Ok(make_bincode_serializer().deserialize(&value.inner)?)
    }
}

#[cfg(test)]
mod tests {

    use nym_credentials_interface::CredentialSpendingData;

    use crate::models::tests::CREDENTIAL_BYTES;

    use self::{
        available_bandwidth::{
            request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
        },
        topup_bandwidth::{request::InnerTopUpRequest, response::InnerTopUpResponse},
    };

    use super::*;

    #[test]
    fn mismatched_request_version() {
        let version = Version::V0;
        let future_bw = Request {
            version,
            inner: vec![],
        };
        if let Err(Error::InvalidVersion {
            source_version,
            target_version,
        }) = VersionedRequest::try_from(future_bw)
        {
            assert_eq!(source_version, version);
            assert_eq!(target_version, VERSION);
        } else {
            panic!("failed");
        };
    }

    #[test]
    fn mismatched_response_version() {
        let version = Version::V0;
        let future_bw = Response {
            version,
            inner: vec![],
        };
        if let Err(Error::InvalidVersion {
            source_version,
            target_version,
        }) = VersionedResponse::try_from(future_bw)
        {
            assert_eq!(source_version, version);
            assert_eq!(target_version, VERSION);
        } else {
            panic!("failed");
        };
    }

    #[test]
    fn serde_request_av_bw() {
        let req = VersionedRequest {
            query_type: QueryType::AvailableBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerAvailableBandwidthResponse {
                    available_bandwidth: 42,
                })
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
                .serialize(&InnerAvailableBandwidthRequest {})
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
            query_type: QueryType::TopupBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerTopUpRequest {
                    credential: CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
                })
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
            query_type: QueryType::TopupBandwidth,
            inner: make_bincode_serializer()
                .serialize(&InnerTopUpResponse {
                    available_bandwidth: 42,
                })
                .unwrap(),
        };

        let ser = Response::try_from(resp.clone()).unwrap();
        assert_eq!(VERSION, ser.version);
        let de = VersionedResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }
}
