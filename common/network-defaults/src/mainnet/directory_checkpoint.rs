// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Compiled-in directory light-client checkpoint (the `SignedCheckpoint` datum, JSON-encoded).
//!
//! This whole file is regenerated wholesale by the offline checkpoint minting tool (see the
//! `directory-checkpoint-bootstrap` change). An empty value means "no compiled checkpoint" -
//! the hardcoded checkpoint provider treats it as absent and the loader falls through to its
//! other sources. It stays empty until the real mainnet checkpoint is minted.

// minted: <none yet - placeholder>
pub const DIRECTORY_CHECKPOINT: &str = "";
