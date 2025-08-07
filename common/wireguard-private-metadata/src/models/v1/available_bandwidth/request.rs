// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::models::Request;

use super::super::{Error, QueryType, VersionedRequest};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InnerAvailableBandwidthRequest {}

impl TryFrom<VersionedRequest> for InnerAvailableBandwidthRequest {
    type Error = Error;

    fn try_from(value: VersionedRequest) -> Result<Self, Self::Error> {
        match value.query_type {
            QueryType::AvailableBandwidth => Ok(bincode::deserialize(&value.inner)?),
            QueryType::TopupBandwidth => Err(Error::InvalidQueryType {
                source_query_type: value.query_type.to_string(),
                target_query_type: QueryType::AvailableBandwidth.to_string(),
            }),
        }
    }
}

impl TryFrom<InnerAvailableBandwidthRequest> for VersionedRequest {
    type Error = Error;

    fn try_from(value: InnerAvailableBandwidthRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            query_type: QueryType::AvailableBandwidth,
            inner: bincode::serialize(&value)?,
        })
    }
}

impl TryFrom<Request> for InnerAvailableBandwidthRequest {
    type Error = crate::error::Error;

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        VersionedRequest::try_from(value)?
            .try_into()
            .map_err(|err: Error| crate::error::Error::Models {
                message: err.to_string(),
            })
    }
}

impl TryFrom<InnerAvailableBandwidthRequest> for Request {
    type Error = crate::error::Error;

    fn try_from(value: InnerAvailableBandwidthRequest) -> Result<Self, Self::Error> {
        VersionedRequest::try_from(value)?
            .try_into()
            .map_err(|err: Error| crate::error::Error::Models {
                message: err.to_string(),
            })
    }
}

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
