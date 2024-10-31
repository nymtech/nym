// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct EpochIdParam {
    pub(super) epoch_id: Option<u64>,
}
