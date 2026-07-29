// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The nym-node subsystem that publishes this node's signed entries to the directory
//! contract. See the `node-directory-publishing` change for the full design.

pub(crate) mod payload;

pub(crate) use payload::DirectoryPayload;
