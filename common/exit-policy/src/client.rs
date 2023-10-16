// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::policy::PolicyError;
use crate::ExitPolicy;
use reqwest::IntoUrl;

pub async fn get_exit_policy(url: impl IntoUrl) -> Result<ExitPolicy, PolicyError> {
    ExitPolicy::parse_from_torrc(reqwest::get(url).await?.text().await?)
}
