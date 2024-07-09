// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod old_config_1_1_2;
mod old_config_1_1_3;

pub use old_config_1_1_2::try_upgrade_config_1_1_2;
pub use old_config_1_1_3::try_upgrade_config_1_1_3;
