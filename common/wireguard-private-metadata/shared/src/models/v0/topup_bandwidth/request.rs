// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bincode::Options;
use serde::{Deserialize, Serialize};

use crate::{make_bincode_serializer, models::Request};

use super::super::{Error, QueryType, VersionedRequest};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerTopUpRequest {}

impl TryFrom<VersionedRequest> for InnerTopUpRequest {
    type Error = Error;

    fn try_from(value: VersionedRequest) -> Result<Self, Self::Error> {
        match value.query_type {
            QueryType::TopupBandwidth => Ok(make_bincode_serializer().deserialize(&value.inner)?),
            QueryType::AvailableBandwidth => Err(Error::InvalidQueryType {
                source_query_type: value.query_type.to_string(),
                target_query_type: QueryType::TopupBandwidth.to_string(),
            }),
        }
    }
}

impl TryFrom<InnerTopUpRequest> for VersionedRequest {
    type Error = Error;

    fn try_from(value: InnerTopUpRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            query_type: QueryType::TopupBandwidth,
            inner: make_bincode_serializer().serialize(&value)?,
        })
    }
}

impl TryFrom<Request> for InnerTopUpRequest {
    type Error = crate::error::MetadataError;

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        VersionedRequest::try_from(value)?
            .try_into()
            .map_err(|err: Error| crate::error::MetadataError::Models {
                message: err.to_string(),
            })
    }
}

impl TryFrom<InnerTopUpRequest> for Request {
    type Error = crate::error::MetadataError;

    fn try_from(value: InnerTopUpRequest) -> Result<Self, Self::Error> {
        VersionedRequest::try_from(value)?
            .try_into()
            .map_err(|err: Error| crate::error::MetadataError::Models {
                message: err.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let req = InnerTopUpRequest {};
        let ser = VersionedRequest::try_from(req.clone()).unwrap();
        assert_eq!(QueryType::TopupBandwidth, ser.query_type);
        let de = InnerTopUpRequest::try_from(ser).unwrap();
        assert_eq!(req, de);
    }

    #[test]
    fn empty_content() {
        let future_req = VersionedRequest {
            query_type: QueryType::TopupBandwidth,
            inner: vec![],
        };
        assert!(InnerTopUpRequest::try_from(future_req).is_ok());
    }
}
