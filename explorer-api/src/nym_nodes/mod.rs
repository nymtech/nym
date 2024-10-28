// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

pub(crate) mod http;
pub(crate) mod location;
pub(crate) mod models;

pub(crate) const CACHE_ENTRY_TTL: Duration = Duration::from_secs(1200);
