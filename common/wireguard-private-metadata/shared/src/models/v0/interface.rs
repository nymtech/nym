// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{
    QueryType, VERSION, VersionedRequest, VersionedResponse,
    available_bandwidth::{
        request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
    },
    topup_bandwidth::{request::InnerTopUpRequest, response::InnerTopUpResponse},
};
use crate::models::{Construct, Extract, Version, error::Error};
use crate::{Request, Response};

#[derive(Debug, Clone, PartialEq)]
pub enum RequestData {
    AvailableBandwidth(()),
    TopUpBandwidth(()),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseData {
    AvailableBandwidth(()),
    TopUpBandwidth(()),
}

impl Construct<RequestData> for VersionedRequest {
    fn construct(info: RequestData, _version: Version) -> Result<Self, Error> {
        match info {
            RequestData::AvailableBandwidth(_) => Ok(InnerAvailableBandwidthRequest {}.try_into()?),
            RequestData::TopUpBandwidth(_) => Ok(InnerTopUpRequest {}.try_into()?),
        }
    }
}

impl Extract<RequestData> for VersionedRequest {
    fn extract(&self) -> Result<(RequestData, Version), Error> {
        match self.query_type {
            QueryType::AvailableBandwidth => {
                let _req = InnerAvailableBandwidthRequest::try_from(self)?;
                Ok((RequestData::AvailableBandwidth(()), VERSION))
            }
            QueryType::TopUpBandwidth => {
                let _req = InnerTopUpRequest::try_from(self)?;
                Ok((RequestData::TopUpBandwidth(()), VERSION))
            }
        }
    }
}

impl Construct<ResponseData> for VersionedResponse {
    fn construct(info: ResponseData, _version: Version) -> Result<Self, Error> {
        match info {
            ResponseData::AvailableBandwidth(()) => {
                Ok(InnerAvailableBandwidthResponse {}.try_into()?)
            }
            ResponseData::TopUpBandwidth(()) => Ok(InnerTopUpResponse {}.try_into()?),
        }
    }
}

impl Extract<ResponseData> for VersionedResponse {
    fn extract(&self) -> Result<(ResponseData, Version), Error> {
        match self.query_type {
            QueryType::AvailableBandwidth => {
                let _resp = InnerAvailableBandwidthResponse::try_from(self)?;
                Ok((ResponseData::AvailableBandwidth(()), VERSION))
            }
            QueryType::TopUpBandwidth => {
                let _resp = InnerTopUpResponse::try_from(self)?;
                Ok((ResponseData::TopUpBandwidth(()), VERSION))
            }
        }
    }
}

#[cfg(feature = "testing")]
impl Extract<RequestData> for Request {
    fn extract(&self) -> Result<(RequestData, Version), Error> {
        match self.version {
            Version::V0 => {
                let versioned_request = VersionedRequest::try_from(self)?;
                versioned_request.extract()
            }
            _ => Err(Error::UpdateNotPossible {
                from: self.version,
                to: VERSION,
            }),
        }
    }
}

#[cfg(feature = "testing")]
impl Construct<ResponseData> for Response {
    fn construct(info: ResponseData, version: Version) -> Result<Self, Error> {
        match version {
            Version::V0 => {
                let translate_response = info;
                let versioned_response = VersionedResponse::construct(translate_response, version)?;
                Ok(versioned_response.try_into()?)
            }
            _ => Err(Error::DowngradeNotPossible {
                from: version,
                to: VERSION,
            }),
        }
    }
}
