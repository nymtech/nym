// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::{init, run};
use crate::config::Config;

pub trait ConfigOverride {
    fn override_config(self, config: &mut Config);
}

impl ConfigOverride for init::ConfigOverridableArgs {
    fn override_config(self, config: &mut Config) {}
}

impl ConfigOverride for run::Args {
    fn override_config(self, config: &mut Config) {}
}
