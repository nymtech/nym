// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

// aliases for backwards compatibility
pub use v1 as old_config_v1_1_13;
pub use v2 as old_config_v1_1_20;
pub use v3 as old_config_v1_1_20_2;
pub use v4 as old_config_v1_1_30;
pub use v5 as old_config_v1_1_33;
