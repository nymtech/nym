// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod keybindings;

pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(200);

const DEFAULT_SHUTDOWN_GRACE: Duration = Duration::from_millis(500);
const DEFAULT_CANCEL_GRACE: Duration = Duration::from_millis(500);
const DEFAULT_ABORT_GRACE: Duration = Duration::from_millis(200);

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct TuiConfig {
    pub tui: Tui,

    // #[serde(default)]
    // pub key_bindings: KeyBindings,
    #[serde(default)]
    pub debug: Debug,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Debug {
    pub debug_mode_enabled: bool,

    #[serde(with = "humantime_serde")]
    pub shutdown_grace: Duration,

    #[serde(with = "humantime_serde")]
    pub cancel_grace: Duration,

    #[serde(with = "humantime_serde")]
    pub abort_grace: Duration,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            debug_mode_enabled: true,
            shutdown_grace: DEFAULT_SHUTDOWN_GRACE,
            cancel_grace: DEFAULT_CANCEL_GRACE,
            abort_grace: DEFAULT_ABORT_GRACE,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Tui {
    #[serde(default, flatten)]
    pub debug: TuiDebug,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct TuiDebug {
    #[serde(with = "humantime_serde")]
    pub tick_rate: Duration,
}

impl Default for TuiDebug {
    fn default() -> Self {
        TuiDebug {
            tick_rate: DEFAULT_TICK_RATE,
            // frame_rate: DEFAULT_FRAME_RATE,
        }
    }
}
