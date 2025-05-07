// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_http_api_common::Output;

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub(super) struct EpochIdParam {
    pub(super) epoch_id: Option<u64>,
    pub(super) output: Option<Output>,
}
