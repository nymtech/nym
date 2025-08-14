// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bincode::Options;
use serde::{Deserialize, Serialize};

use crate::{make_bincode_serializer, models::Response};

use super::super::{Error, QueryType, VersionedResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerTopUpResponse {
    pub available_bandwidth: i64,
}

impl TryFrom<VersionedResponse> for InnerTopUpResponse {
    type Error = Error;

    fn try_from(value: VersionedResponse) -> Result<Self, Self::Error> {
        match value.query_type {
            QueryType::TopupBandwidth => Ok(make_bincode_serializer().deserialize(&value.inner)?),
            QueryType::AvailableBandwidth => Err(Error::InvalidQueryType {
                source_query_type: value.query_type.to_string(),
                target_query_type: QueryType::TopupBandwidth.to_string(),
            }),
        }
    }
}

impl TryFrom<InnerTopUpResponse> for VersionedResponse {
    type Error = Error;

    fn try_from(value: InnerTopUpResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            query_type: QueryType::TopupBandwidth,
            inner: make_bincode_serializer().serialize(&value)?,
        })
    }
}

impl TryFrom<Response> for InnerTopUpResponse {
    type Error = crate::error::MetadataError;

    fn try_from(value: Response) -> Result<Self, Self::Error> {
        VersionedResponse::try_from(value)?
            .try_into()
            .map_err(|err: Error| crate::error::MetadataError::Models {
                message: err.to_string(),
            })
    }
}

impl TryFrom<InnerTopUpResponse> for Response {
    type Error = crate::error::MetadataError;

    fn try_from(value: InnerTopUpResponse) -> Result<Self, Self::Error> {
        VersionedResponse::try_from(value)?
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
        let resp = InnerTopUpResponse {
            available_bandwidth: 42,
        };
        let ser = VersionedResponse::try_from(resp.clone()).unwrap();
        assert_eq!(QueryType::TopupBandwidth, ser.query_type);
        let de = InnerTopUpResponse::try_from(ser).unwrap();
        assert_eq!(resp, de);
    }

    #[test]
    fn invalid_content() {
        let future_resp = VersionedResponse {
            query_type: QueryType::TopupBandwidth,
            inner: vec![],
        };
        assert!(InnerTopUpResponse::try_from(future_resp).is_err());
    }
}
