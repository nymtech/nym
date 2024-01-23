// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::OnceLock;

pub(crate) fn params() -> &'static nym_dkg::bte::Params {
    static PARAMS: OnceLock<nym_dkg::bte::Params> = OnceLock::new();
    PARAMS.get_or_init(nym_dkg::bte::setup)
}

pub(crate) mod client;
pub(crate) mod complaints;
pub(crate) mod controller;
pub(crate) mod dealing;
pub(crate) mod key_derivation;
mod key_finalization;
mod key_validation;
pub(crate) mod public_key;
pub(crate) mod state;
