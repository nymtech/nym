// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{CodedError, SmartQueryError};
use crate::serde_helpers::{de_i64_from_str, de_paged_query_response_from_str};
use mixnet_contract::IdentityKey;
use serde::{Deserialize, Serialize};

// TODO: this is a duplicate code but it really does not feel
// like it would belong in the common crate because it's TOO contract specific...
// I'm not entirely sure what to do about it now.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum QueryRequest {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<IdentityKey>,
    },
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    LayerDistribution {},
}

#[derive(Deserialize, Debug)]
#[serde(bound = "for<'a> T: Deserialize<'a>")]
pub(super) struct SmartQueryResult<T>
where
    for<'a> T: Deserialize<'a>,
{
    #[serde(deserialize_with = "de_paged_query_response_from_str")]
    pub(super) smart: T,
}

#[derive(Deserialize, Debug)]
#[serde(bound = "for<'a> T: Deserialize<'a>")]
pub(super) struct SmartQueryResponse<T>
where
    for<'a> T: Deserialize<'a>,
{
    #[serde(deserialize_with = "de_i64_from_str")]
    pub(super) height: i64,
    pub(super) result: SmartQueryResult<T>,
}

#[derive(Deserialize, Debug)]
#[serde(bound = "for<'a> T: Deserialize<'a>")]
#[serde(untagged)]
pub(super) enum QueryResponse<T>
where
    for<'a> T: Deserialize<'a>,
{
    Ok(SmartQueryResponse<T>),
    Error(SmartQueryError),
    CodedError(CodedError),
}
