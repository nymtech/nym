// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::BlockHeight;

// presumably everything here should only be changed via governance
pub const CREDENTIAL_THRESHOLD: usize = 3;
pub const NUMBER_OF_ATTRIBUTES: usize = 2;

pub const EPOCH_TRANSITION_LENGTH: BlockHeight = 1000;

// perhaps some constant to keep track of when the above were last updated so issuers would know
// if they need to change something? Not entirely sure how that would work just yet.
pub const LAST_UPDATED: usize = 0;

// another idea to store 'epoch' so that whenever it changes, all other validators would have to
// recreate their keys?
pub const EPOCH: usize = 1;
