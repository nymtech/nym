// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_http_api_common::Output;

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub(super) struct EpochIdParam {
    /// The DKG epoch whose material this request concerns. Defaults to the epoch currently in
    /// service.
    ///
    /// Worth stating explicitly whenever a request is one of several being collected for the same
    /// credential: a ceremony concluding partway through moves the default, and material from two
    /// different epochs cannot be combined.
    pub(super) epoch_id: Option<u64>,
    pub(super) output: Option<Output>,
}
