// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::Epoch;

pub mod storage;

pub(crate) struct State {
    current_epoch: Epoch,

    // keep track of the next epoch if it's in the process of sharing dealings, etc.
    upcoming_epoch: Option<Epoch>,
}
