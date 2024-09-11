// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod old_config_v1;
mod old_config_v2;
mod old_config_v3;

pub use old_config_v1::try_upgrade_config_v1;
pub use old_config_v2::try_upgrade_config_v2;
pub use old_config_v3::try_upgrade_config_v3;
