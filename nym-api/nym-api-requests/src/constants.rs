// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use time::Duration;

// we should probably monitor this constant and adjust it when/if required
pub const MIN_BATCH_REDEMPTION_DELAY: Duration = Duration::DAY;
