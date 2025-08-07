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

pub(crate) mod available_bandwidth;
pub(crate) mod interface;
pub(crate) mod topup_bandwidth;

pub const VERSION: Version = Version::V0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum QueryType {
    AvailableBandwidth,
    TopupBandwidth,
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
    use crate::{client::WireguardMetadataApiClient, tests::spawn_server_and_create_client};

    use self::{
        available_bandwidth::{
            request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
        },
        topup_bandwidth::request::InnerTopUpRequest,
    };

    use super::*;

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
                .serialize(&InnerAvailableBandwidthRequest {})
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
                .serialize(&InnerAvailableBandwidthRequest {})
                .unwrap(),
        };

        let ser = Response::try_from(resp.clone()).unwrap();
        assert_eq!(VERSION, ser.version);
        let de = VersionedResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }

    #[tokio::test]
    async fn query_available_bandwidth() {
        let client = spawn_server_and_create_client().await;
        let request = InnerAvailableBandwidthRequest {}.try_into().unwrap();

        let response = client.available_bandwidth(&request).await.unwrap();

        InnerAvailableBandwidthResponse::try_from(response).unwrap();
    }

    #[tokio::test]
    async fn query_topup_bandwidth() {
        let client = spawn_server_and_create_client().await;
        let request = InnerTopUpRequest {}.try_into().unwrap();

        // topup no longer possible with latest version (needs credential)
        assert!(client.topup_bandwidth(&request).await.is_err());
    }
}
