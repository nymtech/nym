// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct LocalnetCache {
    cache_dir: PathBuf,
}

impl LocalnetCache {
    pub(crate) fn new<P: AsRef<Path>>(cache_dir: P) -> anyhow::Result<Self> {
        let cache_dir = cache_dir.as_ref();

        let this = Self {
            cache_dir: cache_dir.to_path_buf(),
        };

        // make sure all paths exist
        fs::create_dir_all(cache_dir)?;
        fs::create_dir_all(this.contracts_directory())?;
        fs::create_dir_all(this.kernel_configs_directory())?;

        Ok(this)
    }

    pub(crate) fn contracts_directory(&self) -> PathBuf {
        self.cache_dir.join("contracts")
    }

    pub(crate) fn kernel_configs_directory(&self) -> PathBuf {
        self.cache_dir.join("kernels")
    }

    pub(crate) fn cached_contract_path(&self, contract_filename: &str) -> PathBuf {
        self.contracts_directory().join(contract_filename)
    }

    pub(crate) fn cached_contract_exists(&self, contract_filename: &str) -> bool {
        self.cached_contract_path(contract_filename).exists()
    }

    pub(crate) fn clear(&self) -> anyhow::Result<()> {
        fs::remove_dir_all(&self.cache_dir)?;
        Ok(())
    }
}
