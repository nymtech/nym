// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

pub(crate) mod http;
pub(crate) mod location;
pub(crate) mod models;
pub(crate) mod utils;

pub(crate) const CACHE_REFRESH_RATE: Duration = Duration::from_secs(30);
pub(crate) const CACHE_ENTRY_TTL: Duration = Duration::from_secs(60);
