// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::CommonClientPaths;
use crate::error::ClientCoreError;
use std::path::{Path, PathBuf};

// we can suppress this warning (as suggested by linter itself) since we're only using it in our own code
#[allow(async_fn_in_trait)]
pub trait CliClient {
    const NAME: &'static str;
    type Error: From<ClientCoreError>;
    type Config: CliClientConfig;

    async fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error>;

    async fn try_load_current_config(id: &str) -> Result<Self::Config, Self::Error>;
}

pub trait CliClientConfig {
    fn common_paths(&self) -> &CommonClientPaths;

    fn core_config(&self) -> &crate::config::Config;

    fn default_store_location(&self) -> PathBuf;

    fn save_to<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()>;
}
