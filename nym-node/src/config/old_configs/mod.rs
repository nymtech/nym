// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod old_config_v1;
mod old_config_v2;
mod old_config_v3;
mod old_config_v4;
mod old_config_v5;
mod old_config_v6;
mod old_config_v7;
mod old_config_v8;

pub use old_config_v1::try_upgrade_config_v1;
pub use old_config_v2::try_upgrade_config_v2;
pub use old_config_v3::try_upgrade_config_v3;
pub use old_config_v4::try_upgrade_config_v4;
pub use old_config_v5::try_upgrade_config_v5;
pub use old_config_v6::try_upgrade_config_v6;
pub use old_config_v7::try_upgrade_config_v7;
pub use old_config_v8::try_upgrade_config_v8;
