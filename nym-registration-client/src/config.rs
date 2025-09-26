// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_registration_common::NymNode;
use std::path::PathBuf;

pub struct RegistrationClientConfig {
    pub(crate) entry: NymNode,
    pub(crate) exit: NymNode,
    pub(crate) two_hops: bool,
    pub(crate) data_path: Option<PathBuf>,
}
