// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given mix is
/// currently up or down (based on whether it's mixing packets)
pub struct MixStatus {
    pub pub_key: String,
    pub owner: String,
    pub ip_version: String,
    pub up: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given set of mixes is
/// currently up or down (based on whether it's mixing packets)
pub struct BatchMixStatus {
    pub status: Vec<MixStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given gateway is
/// currently up or down (based on whether it's mixing packets)
pub struct GatewayStatus {
    pub pub_key: String,
    pub owner: String,
    pub ip_version: String,
    pub up: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given set of gateways is
/// currently up or down (based on whether it's mixing packets)
pub struct BatchGatewayStatus {
    pub status: Vec<GatewayStatus>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum ErrorResponses {
    Error(ErrorResponse),
    Unexpected(serde_json::Value),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OkResponse {
    pub(crate) ok: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum DefaultRestResponse {
    Ok(OkResponse),
    Error(ErrorResponses),
}
