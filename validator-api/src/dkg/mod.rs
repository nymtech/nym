// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod dealing_processing;
pub(crate) mod error;
pub(crate) mod events;
mod main_loop;
pub(crate) mod networking;
mod smart_contract;
pub(crate) mod state;

// upon startup, the following tasks will need to be spawned:
// - smart contract watcher
// - main loop processing
// - dealing processor
// - network listener
// - event dispatcher
// (possibly): network sender (if listens for events, otherwise under control of main loop)
// (possibly): contract publisher (if listens for events, otherwise under control of main loop)
