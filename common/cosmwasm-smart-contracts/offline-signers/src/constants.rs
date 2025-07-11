// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;

pub const DEFAULT_REQUIRED_QUORUM: Decimal = Decimal::percent(50);

pub const DEFAULT_MAXIMUM_PROPOSAL_LIFETIME_SECS: u64 = 4 * 60 * 60; // 4h

pub const DEFAULT_STATUS_CHANGE_COOLDOWN_SECS: u64 = 300; // 5min

pub mod storage_keys {
    pub const CONTRACT_ADMIN: &str = "contract-admin";
    pub const DKG_CONTRACT: &str = "dkg_contract";
    pub const CONFIG: &str = "config";
    pub const ACTIVE_PROPOSALS: &str = "active_proposals";
    pub const PROPOSALS: &str = "proposals";
    pub const VOTES: &str = "votes";
    pub const OFFLINE_SIGNERS_INFORMATION: &str = "offline_signers_information";
    pub const OFFLINE_SIGNERS: &str = "offline_signers";
    pub const OFFLINE_SIGNERS_CHECKPOINTS: &str = "offline_signers__check";
    pub const OFFLINE_SIGNERS_CHANGELOG: &str = "offline_signers__change";
    pub const LAST_STATUS_RESET: &str = "last_status_reset";
    pub const PROPOSAL_COUNT: &str = "proposal_count";
}
