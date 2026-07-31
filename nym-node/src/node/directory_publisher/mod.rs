// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The nym-node subsystem that publishes this node's signed entries to the directory
//! contract. See the `node-directory-publishing` change for the full design.

// TODO: this should be some global value instead
const DEFAULT_MINIMUM_ON_CHAIN_BALANCE_AMOUNT: u128 = 1_000000; // 1 nym is enough for all tx fees for quite some time

pub(crate) mod payload;
mod preflight;
mod publisher;
mod session;
#[cfg(test)]
pub(crate) mod test_utils;
pub(crate) mod traits;

pub(crate) use payload::DirectoryPayload;
pub(crate) use publisher::{
    DirectoryPublisher, DirectoryPublisherConfig, DirectoryPublisherEventsSender,
};
pub(crate) use traits::DirectoryChainClient;
