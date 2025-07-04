// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod account;
mod device;
mod gateway;
mod platform;

#[cfg(test)]
mod test_fixtures;

pub use account::{Error as AccountError, VpnApiAccount, VpnApiTime, VpnApiTimeSynced};
pub use device::{Device, DeviceStatus};
pub use gateway::{GatewayMinPerformance, GatewayType, ScoreThresholds};
pub use platform::Platform;

pub use nym_contracts_common::{NaiveFloat, Percent};
