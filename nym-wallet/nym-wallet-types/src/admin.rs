// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_types::currency::DecCoin;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/StateParams.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriContractStateParams {
    minimum_pledge: DecCoin,
    minimum_delegation: Option<DecCoin>,

    operating_cost: TauriOperatingCostRange,
    profit_margin: TauriProfitMarginRange,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/OperatingCostRange.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriOperatingCostRange {
    minimum: DecCoin,
    maximum: DecCoin,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/ProfitMarginRange.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriProfitMarginRange {
    minimum: String,
    maximum: String,
}
