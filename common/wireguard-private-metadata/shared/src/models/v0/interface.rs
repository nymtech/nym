// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{
    available_bandwidth::{
        request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
    },
    topup_bandwidth::{request::InnerTopUpRequest, response::InnerTopUpResponse},
    QueryType, VersionedRequest, VersionedResponse, VERSION,
};
use crate::models::{error::Error, Construct, Extract, Version};

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
                let _req = InnerAvailableBandwidthRequest::try_from(self.clone())?;
                Ok((RequestData::AvailableBandwidth(()), VERSION))
            }
            QueryType::TopupBandwidth => {
                let _req = InnerTopUpRequest::try_from(self.clone())?;
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
                let _resp = InnerAvailableBandwidthResponse::try_from(self.clone())?;
                Ok((ResponseData::AvailableBandwidth(()), VERSION))
            }
            QueryType::TopupBandwidth => {
                let _resp = InnerTopUpResponse::try_from(self.clone())?;
                Ok((ResponseData::TopUpBandwidth(()), VERSION))
            }
        }
    }
}
