// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_authenticator_requests::traits::CurrentUpgradeModeStatus;

#[derive(Debug, Clone, Copy)]
pub struct TopUpClientResponse {
    pub remaining_bandwidth_bytes: i64,
    pub current_upgrade_mode_status: CurrentUpgradeModeStatus,
}

#[derive(Debug, Clone, Copy)]
pub struct AvailableBandwidthClientResponse {
    pub available_bandwidth_bytes: Option<i64>,
    pub current_upgrade_mode_status: CurrentUpgradeModeStatus,
}
