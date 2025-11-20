// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTAINER_NETWORK_NAME;
use crate::helpers::{exec_inherit_output, monorepo_root_path};
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::container_helpers::{
    attach_run_container_args, container_binary, default_nym_binaries_image_tag,
};
use anyhow::Context;
use bip39::Mnemonic;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

impl LocalnetOrchestrator {
    fn make_global_env_file(&self) -> anyhow::Result<()> {
        let path = self.storage.global_env_file();
        if path.exists() {
            return Ok(());
        }
        let content = self.localnet_details.env_file_content()?;
        fs::write(path, &content)?;
        Ok(())
    }

    async fn start_gateway_probe(
        &self,
        monorepo_root: &Path,
        mnemonic: &Mnemonic,
        additional_args: Option<String>,
    ) -> anyhow::Result<()> {
        // run this instance with piped output so we could see live changes
        let bin = container_binary();

        let monorepo_path = monorepo_root.canonicalize()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;

        // first we construct the base, common, args
        let env_file_volume = format!(
            "{}:/root",
            self.storage
                .global_env_file()
                .parent()
                .context("invalid storage dir")?
                .canonicalize()?
                .to_string_lossy()
        );
        let mnemonic_string = mnemonic.to_string();
        let mut probe_args = vec![
            "-v".to_string(),
            env_file_volume,
            "--network".to_string(),
            CONTAINER_NETWORK_NAME.to_string(),
            "--rm".to_string(),
            image_tag,
            "nym-gateway-probe".to_string(),
            "-c".to_string(),
            "/root/localnet.env".to_string(),
            "run-local".to_string(),
            "--mnemonic".to_string(),
            mnemonic_string,
        ];
        if let Some(additional_args) = additional_args {
            probe_args.push(additional_args)
        }

        // then we attach platform specific ones
        let mut probe_args = attach_run_container_args(probe_args);

        // finally we insert the "run" at the beginning
        probe_args.insert(0, "run".into());

        info!("🚀🚀🚀 STARTING THE GATEWAY PROBE");
        exec_inherit_output(bin, probe_args).await?;
        Ok(())
    }

    pub(crate) async fn run_gateway_probe(
        &self,
        monorepo_root: Option<PathBuf>,
        additional_args: Option<String>,
    ) -> anyhow::Result<()> {
        let monorepo_root = monorepo_root_path(monorepo_root)?;

        // 1. create env file
        self.make_global_env_file()?;

        // 2. retrieve admin account (no point in making a new one - this one has plenty of tokens)
        let account = &self.localnet_details.nyxd_details()?.master_account;

        // 3. run the actual probe
        self.start_gateway_probe(&monorepo_root, &account.mnemonic, additional_args)
            .await?;

        Ok(())
    }
}
