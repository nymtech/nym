// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum NetworkStatisticsAPIError {
    #[error("{0}")]
    RocketError(#[from] rocket::Error),
}
