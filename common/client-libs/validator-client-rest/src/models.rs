// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::serde_helpers::{de_i64_from_str, de_paged_query_response_from_str};
use core::fmt::{self, Display, Formatter};
use mixnet_contract::HumanAddr;
use serde::{Deserialize, Serialize};

// TODO: this is a duplicate code but it really does not feel
// like it would belong in the common crate because it's TOO contract specific...
// I'm not entirely sure what to do about it now.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum QueryRequest {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<HumanAddr>,
    },
    GetGateways {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
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
pub(super) struct SmartQueryError {
    pub(super) error: String,
}

// this is the case of message like
/*
{
  "code": 12,
  "message": "Not Implemented",
  "details": [
  ]
}
 */
// I didn't manage to find where it exactly originates, nor what the correct types should be
// so all of those are some educated guesses
#[derive(Deserialize, Debug)]
pub(super) struct CodedError {
    code: u32,
    message: String,
    details: Vec<(String, String)>,
}

impl Display for CodedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "code: {} - {}", self.code, self.message)?;
        // this is under assumption that details are indeed key value pairs which is a big IF
        for detail in &self.details {
            write!(f, " {:?}", detail)?
        }
        Ok(())
    }
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
