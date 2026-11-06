// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

pub(crate) mod handlers;
pub(crate) mod helpers;
pub(crate) mod models;

pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);
