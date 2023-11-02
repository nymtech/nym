// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UpgradePlan {
    //
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpgradeInfo {
    //
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpgradeHistory(Vec<UpgradeHistoryEntry>);

#[derive(Serialize, Deserialize, Debug)]
pub struct UpgradeHistoryEntry {
    performed_at: u64,
    info: UpgradeInfo,
}
