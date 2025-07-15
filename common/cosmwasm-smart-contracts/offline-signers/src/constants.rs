// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod storage_keys {
    pub const CONTRACT_ADMIN: &str = "contract-admin";
    pub const DKG_CONTRACT: &str = "dkg_contract";
    pub const CONFIG: &str = "config";
    pub const ACTIVE_PROPOSALS: &str = "active_proposals";
    pub const PROPOSALS: &str = "proposals";
    pub const VOTES: &str = "votes";
    pub const OFFLINE_SIGNERS_PRIMARY: &str = "offline_signers";
    pub const OFFLINE_SIGNERS_CHECKPOINTS: &str = "offline_signers__check";
    pub const OFFLINE_SIGNERS_CHANGELOG: &str = "offline_signers__change";
    pub const PROPOSAL_COUNT: &str = "proposal_count";
}
