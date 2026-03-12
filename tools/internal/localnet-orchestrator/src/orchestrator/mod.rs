// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::constants::CONTAINER_NETWORK_NAME;
use crate::helpers::{exec_fallible_cmd_with_output, generate_network_name};
use crate::orchestrator::container_helpers::{
    create_container_network, is_container_network_running, run_container,
};
use crate::orchestrator::context::{LocalnetContext, ephemeral_context};
use crate::orchestrator::network::Localnet;
use crate::orchestrator::state::LocalnetState;
use crate::orchestrator::storage::orchestrator::LocalnetOrchestratorStorage;
use crate::orchestrator::storage::{
    LocalnetStorage, default_cache_dir, default_orchestrator_db_file, default_storage_dir,
};
use anyhow::{Context, bail};
use std::collections::HashMap;
use std::env::temp_dir;
use std::fs;
use tracing::info;

pub mod account;
pub(crate) mod container_helpers;
pub(crate) mod context;
pub(crate) mod cosmwasm_contract;
pub(crate) mod helpers;
pub(crate) mod network;
pub(crate) mod nym_node;
pub(crate) mod setup;
pub(crate) mod state;
pub(crate) mod storage;
pub(crate) mod test_cmds;

pub(crate) struct LocalnetOrchestrator {
    pub(crate) state: LocalnetState,

    pub(crate) localnet_details: Localnet,
    pub(crate) storage: LocalnetStorage,
}

impl LocalnetOrchestrator {
    pub(crate) async fn new(args: &CommonArgs) -> anyhow::Result<Self> {
        let orchestrator_data = args
            .orchestrator_db
            .clone()
            .unwrap_or_else(default_orchestrator_db_file);
        let orchestrator_storage = LocalnetOrchestratorStorage::init(orchestrator_data).await?;

        // if network name has not been explicitly provided, we use the latest one created
        // or if this is the first one, we generate a new one
        let network_name = match args.existing_network.clone() {
            // name provided => see if it existed
            Some(network_name) => {
                // sanity check: try to load metadata (it will fail if entry does not exist)
                let _ = orchestrator_storage
                    .get_localnet_metadata_by_name(&network_name)
                    .await?;
                network_name
            }
            // name not provided
            None => {
                let metadata = orchestrator_storage.get_last_created().await?;
                // have we initialised anything before?
                match metadata.latest_network_id {
                    // no => create new entry
                    None => {
                        let network_name = generate_network_name();
                        orchestrator_storage
                            .save_new_localnet_metadata(&network_name)
                            .await?;
                        network_name
                    }
                    // yes => attempt to retrieve it
                    Some(localnet_id) => {
                        orchestrator_storage
                            .get_localnet_metadata(localnet_id)
                            .await?
                            .name
                    }
                }
            }
        };

        let localnet_directory = match args.localnet_storage_path.clone() {
            Some(localnet_storage_path) => localnet_storage_path,
            None => {
                if args.ephemeral {
                    temp_dir().join(&network_name)
                } else {
                    default_storage_dir().join(&network_name)
                }
            }
        };

        info!("setting up network '{network_name}'");
        info!("main storage directory: '{}'", localnet_directory.display());

        let cache_dir = default_cache_dir();

        let mut this = LocalnetOrchestrator {
            state: Default::default(),
            storage: LocalnetStorage::new(localnet_directory, cache_dir, orchestrator_storage)?,
            localnet_details: Localnet::new(network_name),
        };
        let ctx = ephemeral_context("performing initial state check...");

        this.check_system_deps().await?;
        this.check_kernel_config(&ctx).await?;
        this.resync_state(&ctx).await?;

        info!("initial state: {}", this.state);

        // pre-requirements for any subsequent command
        this.create_localnet_network_if_doesnt_exist().await?;
        Ok(this)
    }

    async fn check_kernel_config(&self, ctx: &LocalnetContext) -> anyhow::Result<()> {
        // NOTE: this is incomplete, I haven't yet determined full set of required config values
        const REQUIRED_CONFIG: &[(&str, &str)] = &[("CONFIG_TUN", "y"), ("CONFIG_NF_TABLES", "y")];

        let stdout = run_container(
            ctx,
            [
                "--rm",
                "-v",
                &self.kernel_configs_volume(),
                "busybox:latest",
                "sh",
                "-c",
                r#"
                    mkdir /root/kernel-configs
                    cat /proc/config.gz | gunzip > /root/kernel-configs/"$(uname -r)"
                    uname -r
                "#,
            ],
            None,
        )
        .await?;
        let maybe_kernel = String::from_utf8(stdout).context("malformed kernel version")?;
        info!("found kernel version: {maybe_kernel}");

        // sure, it's easier to check it directly on the machine,
        // but persisting the file locally makes it easier to debug
        let config_values = fs::read_to_string(
            self.storage
                .data_cache()
                .kernel_configs_directory()
                .join(maybe_kernel.trim()),
        )
        .context("failed to read retrieved kernel config")?;

        let mut enabled_configs = HashMap::new();

        for config in config_values.lines().filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        }) {
            let (key, value) = config
                .split_once('=')
                .context(format!("malformed kernel config entry: '{config}'"))?;
            enabled_configs.insert(key, value);
        }

        for (expected_key, expected_value) in REQUIRED_CONFIG {
            let Some(value) = enabled_configs.get(expected_key) else {
                bail!(
                    "{expected_key} not set in the kernel - please either recompile it or obtain a valid image"
                );
            };
            if value != expected_value {
                bail!(
                    "{expected_key} does not have the expected value. we need it to be set to '{expected_value}' but it's set to '{value}'"
                );
            }
            ctx.println_with_emoji(
                format!("{expected_key}={expected_value} present in the kernel"),
                "✅",
            )
        }

        Ok(())
    }

    async fn create_localnet_network_if_doesnt_exist(&self) -> anyhow::Result<()> {
        info!("checking if {CONTAINER_NETWORK_NAME} network exists");

        if !is_container_network_running().await? {
            create_container_network().await?;
        }

        Ok(())
    }

    /// Inspects the current network state and resyncs initial state
    /// for example if there's already a nyxd running, there's no point in redeploying it
    /// (unless forced by the cli)
    async fn resync_state(&mut self, ctx: &LocalnetContext) -> anyhow::Result<()> {
        let latest_nyxd_id = self
            .storage
            .orchestrator()
            .get_last_created()
            .await?
            .latest_nyxd_id;

        if self.check_nyxd_container_is_running(ctx).await? {
            // ASSUMPTION: if container is running it is using the latest initialised nyxd instance
            let latest_nyxd_id = latest_nyxd_id
                .context("nyxd container running, but no known nyxd instances initialised")?;

            let nyxd_details = self
                .storage
                .orchestrator()
                .get_nyxd_details(latest_nyxd_id)
                .await?;
            self.localnet_details.set_nyxd_details(nyxd_details);

            self.state = LocalnetState::RunningNyxd
        } else {
            return Ok(());
        }

        let metadata = self
            .storage
            .orchestrator()
            .get_localnet_metadata_by_name(&self.localnet_details.human_name)
            .await?;

        let maybe_contracts = self
            .storage
            .orchestrator()
            .load_localnet_contracts(metadata.id)
            .await;
        let auxiliary_accounts = self
            .storage
            .orchestrator()
            .load_auxiliary_accounts(metadata.id)
            .await;

        match (maybe_contracts, auxiliary_accounts) {
            (Ok(contracts), Ok(auxiliary_accounts)) => {
                self.localnet_details
                    .set_auxiliary_accounts(auxiliary_accounts)
                    .set_contracts(contracts);
                self.state = LocalnetState::DeployedNymContracts;
            }
            _ => return Ok(()),
        }

        // at this point there is no restarting containers due to changing ips
        if self.check_nym_api_container_is_running(ctx).await? {
            let nym_api = self
                .storage
                .orchestrator()
                .get_nym_api_details(metadata.id)
                .await?;
            self.localnet_details.set_nym_api_endpoint(nym_api);
            self.state = LocalnetState::RunningNymApi;
        } else {
            return Ok(());
        }

        if self.check_nym_node_containers_are_running(ctx).await? {
            self.state = LocalnetState::RunningNymNodes;
        }

        Ok(())
    }

    async fn check_dep_exists(&self, name: &str) -> anyhow::Result<()> {
        if !exec_fallible_cmd_with_output("which", [name])
            .await?
            .status
            .success()
        {
            bail!("'{}' installation not found", name)
        }
        Ok(())
    }

    async fn check_system_deps(&self) -> anyhow::Result<()> {
        self.check_dep_exists("docker").await?;

        cfg_if::cfg_if! {
            if #[cfg(target_os = "macos")] {
                self.check_dep_exists("container").await?;
            } else if #[cfg(target_os = "linux")] {
                self.check_dep_exists("newuidmap").await?;
                self.check_dep_exists("newgidmap").await?;
                self.check_dep_exists("containerd").await?;
                self.check_dep_exists("nerdctl").await?;
                self.check_dep_exists("kata-runtime").await?;
                self.check_dep_exists("containerd-shim-kata-v2").await?;
            }
        }
        Ok(())
    }
}
