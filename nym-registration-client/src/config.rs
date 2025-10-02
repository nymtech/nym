// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::builder::config::NymNodeWithKeys;

pub struct RegistrationClientConfig {
    pub(crate) entry: NymNodeWithKeys,
    pub(crate) exit: NymNodeWithKeys,
    pub(crate) two_hops: bool,
}
