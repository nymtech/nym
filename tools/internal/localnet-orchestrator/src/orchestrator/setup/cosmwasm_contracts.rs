// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::contract_build_names;
use crate::constants::{CARGO_REGISTRY_CACHE_VOLUME, CI_BUILD_SERVER, CONTRACTS_CACHE_VOLUME};
use crate::helpers::{
    download_cosmwasm_contract, monorepo_root_path, nym_cosmwasm_contract_names,
    retrieve_current_nymnode_version,
};
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::account::Account;
use crate::orchestrator::context::LocalnetContext;
use crate::orchestrator::cosmwasm_contract::ContractBeingInitialised;
use crate::orchestrator::network::{AuxiliaryAccounts, NymContractsBeingInitialised};
use crate::orchestrator::state::LocalnetState;
use anyhow::{Context, bail};
use cw_utils::Threshold;
use nym_coconut_dkg_common::types::TimeConfiguration;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::reward_params::RewardedSetParams;
use nym_mixnet_contract_common::{Decimal, InitialRewardingParams};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::cosmwasm_client::types::InstantiateOptions;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::{debug, info};

pub(crate) struct Config {
    pub(crate) reproducible_builds: bool,
    pub(crate) cosmwasm_optimizer_image: String,
    pub(crate) explicit_contracts_directory: Option<PathBuf>,
    pub(crate) ci_build_branch: Option<String>,
    pub(crate) monorepo_root: Option<PathBuf>,
    pub(crate) allow_cached_build: bool,
}

pub(crate) struct ContractsSetup {
    reproducible_builds: bool,
    cosmwasm_optimizer_image: String,
    allow_cached_build: bool,

    contracts_wasm_dir: Option<PathBuf>,
    ci_build_branch: Option<String>,
    monorepo_root: PathBuf,
    contracts: NymContractsBeingInitialised,
    auxiliary_accounts: AuxiliaryAccounts,
}

impl ContractsSetup {
    pub(crate) fn new(config: Config) -> anyhow::Result<Self> {
        let monorepo_root = monorepo_root_path(config.monorepo_root)?;

        Ok(ContractsSetup {
            reproducible_builds: config.reproducible_builds,
            cosmwasm_optimizer_image: config.cosmwasm_optimizer_image,
            allow_cached_build: config.allow_cached_build,
            contracts_wasm_dir: config.explicit_contracts_directory,
            ci_build_branch: config.ci_build_branch,
            contracts: NymContractsBeingInitialised {
                mixnet: ContractBeingInitialised::new("mixnet"),
                vesting: ContractBeingInitialised::new("vesting"),
                ecash: ContractBeingInitialised::new("ecash"),
                cw3_multisig: ContractBeingInitialised::new("cw3-multisig"),
                cw4_group: ContractBeingInitialised::new("cw4-group"),
                dkg: ContractBeingInitialised::new("dkg"),
                performance: ContractBeingInitialised::new("performance"),
            },
            monorepo_root,
            auxiliary_accounts: AuxiliaryAccounts::new(),
        })
    }
}

impl LocalnetOrchestrator {
    fn contract_signer(
        &self,
        contract: &ContractBeingInitialised,
    ) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let mnemonic = &contract.admin()?.mnemonic;
        self.signing_client(mnemonic)
    }

    fn mixnet_migrate_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_mixnet_contract_common::MigrateMsg> {
        Ok(nym_mixnet_contract_common::MigrateMsg {
            vesting_contract_address: Some(ctx.data.contracts.vesting.address()?.to_string()),
            unsafe_skip_state_updates: Some(true),
        })
    }

    fn multisig_migrate_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_multisig_contract_common::msg::MigrateMsg> {
        Ok(nym_multisig_contract_common::msg::MigrateMsg {
            coconut_bandwidth_address: ctx.data.contracts.ecash.address()?.to_string(),
            coconut_dkg_address: ctx.data.contracts.dkg.address()?.to_string(),
        })
    }

    fn mixnet_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_mixnet_contract_common::InstantiateMsg> {
        Ok(nym_mixnet_contract_common::InstantiateMsg {
            rewarding_validator_address: ctx
                .data
                .auxiliary_accounts
                .mixnet_rewarder
                .address()
                .to_string(),
            // PLACEHOLDER \/
            vesting_contract_address: ctx
                .data
                .auxiliary_accounts
                .mixnet_rewarder
                .address()
                .to_string(),
            // PLACEHOLDER /\
            rewarding_denom: "unym".to_string(),
            epochs_in_interval: 720,
            epoch_duration: Duration::from_secs(60 * 60),
            initial_rewarding_params: InitialRewardingParams {
                initial_reward_pool: Decimal::from_atomics(250_000_000_000_000u128, 0)?,
                initial_staking_supply: Decimal::from_atomics(100_000_000_000_000u128, 0)?,
                staking_supply_scale_factor: Percent::from_percentage_value(50)?,
                sybil_resistance: Percent::from_percentage_value(30)?,
                active_set_work_factor: Decimal::from_atomics(10u32, 0)?,
                interval_pool_emission: Percent::from_percentage_value(2)?,
                rewarded_set_params: RewardedSetParams {
                    entry_gateways: 70,
                    exit_gateways: 50,
                    mixnodes: 120,
                    standby: 0,
                },
            },
            current_nym_node_version: retrieve_current_nymnode_version(&ctx.data.monorepo_root)?,
            version_score_weights: Default::default(),
            version_score_params: Default::default(),
            profit_margin: Default::default(),
            interval_operating_cost: Default::default(),
            key_validity_in_epochs: None,
        })
    }

    fn vesting_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_vesting_contract_common::InitMsg> {
        Ok(nym_vesting_contract_common::InitMsg {
            mixnet_contract_address: ctx.data.contracts.mixnet.address()?.to_string(),
            mix_denom: "unym".to_string(),
        })
    }

    fn dkg_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_coconut_dkg_common::msg::InstantiateMsg> {
        Ok(nym_coconut_dkg_common::msg::InstantiateMsg {
            group_addr: ctx.data.contracts.cw4_group.address()?.to_string(),
            multisig_addr: ctx.data.contracts.cw3_multisig.address()?.to_string(),
            time_configuration: Some(TimeConfiguration {
                public_key_submission_time_secs: 3600,
                dealing_exchange_time_secs: 3600,
                verification_key_submission_time_secs: 3600,
                verification_key_validation_time_secs: 3600,
                verification_key_finalization_time_secs: 3600,
                in_progress_time_secs: 10000000000,
            }),
            mix_denom: "unym".to_string(),
            key_size: 5,
        })
    }

    fn ecash_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_ecash_contract_common::msg::InstantiateMsg> {
        Ok(nym_ecash_contract_common::msg::InstantiateMsg {
            holding_account: ctx
                .data
                .auxiliary_accounts
                .ecash_holding_account
                .address
                .to_string(),
            multisig_addr: ctx.data.contracts.cw3_multisig.address()?.to_string(),
            group_addr: ctx.data.contracts.cw4_group.address()?.to_string(),
            deposit_amount: ctx.unym(75_000_000).into(),
        })
    }

    fn cw3_multisig_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_multisig_contract_common::msg::InstantiateMsg> {
        Ok(nym_multisig_contract_common::msg::InstantiateMsg {
            group_addr: ctx.data.contracts.cw4_group.address()?.to_string(),

            // PLACEHOLDER \/
            coconut_bandwidth_contract_address: ctx.data.contracts.cw4_group.address()?.to_string(),
            coconut_dkg_contract_address: ctx.data.contracts.cw4_group.address()?.to_string(),
            // PLACEHOLDER /\
            threshold: Threshold::AbsolutePercentage {
                percentage: "0.67".parse()?,
            },
            max_voting_period: cw_utils::Duration::Time(3600),
            executor: None,
            proposal_deposit: None,
        })
    }

    fn cw4_group_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_group_contract_common::msg::InstantiateMsg> {
        Ok(nym_group_contract_common::msg::InstantiateMsg {
            admin: Some(ctx.data.contracts.cw4_group.admin()?.address().to_string()),
            // TODO: prepopulate
            members: vec![],
        })
    }

    fn performance_init_message(
        &self,
        ctx: &LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<nym_performance_contract_common::msg::InstantiateMsg> {
        Ok(nym_performance_contract_common::msg::InstantiateMsg {
            mixnet_contract_address: ctx.data.contracts.mixnet.address()?.to_string(),
            authorised_network_monitors: vec![
                ctx.data
                    .auxiliary_accounts
                    .network_monitor
                    .iter()
                    .map(|nm| nm.address.to_string())
                    .collect(),
            ],
        })
    }

    fn contracts_wasm_dir(&self, ctx: &LocalnetContext<ContractsSetup>) -> PathBuf {
        if let Some(explicit) = &ctx.data.contracts_wasm_dir {
            return explicit.clone();
        }
        self.storage.cosmwasm_contracts_directory()
    }

    async fn download_cosmwasm_contracts(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        let Some(ci_build_branch) = ctx.data.ci_build_branch.clone() else {
            bail!("no CI branch specified for downloading pre-built contracts")
        };

        ctx.begin_next_step(
            format!("downloading cosmwasm contracts from {CI_BUILD_SERVER}/{ci_build_branch}/..."),
            "⬇️",
        );
        let out_dir = self.contracts_wasm_dir(ctx);
        fs::create_dir_all(&out_dir)?;
        info!("downloading cosmwasm contracts to {}", out_dir.display());

        ctx.set_pb_prefix("[1/7]");
        ctx.set_pb_message("downloading mixnet contract...");
        download_cosmwasm_contract(&out_dir, &ci_build_branch, contract_build_names::MIXNET)
            .await?;

        ctx.set_pb_prefix("[2/7]");
        ctx.set_pb_message("downloading vesting contract...");
        download_cosmwasm_contract(&out_dir, &ci_build_branch, contract_build_names::VESTING)
            .await?;

        ctx.set_pb_prefix("[3/7]");
        ctx.set_pb_message("downloading ecash contract...");
        download_cosmwasm_contract(&out_dir, &ci_build_branch, contract_build_names::ECASH).await?;

        ctx.set_pb_prefix("[4/7]");
        ctx.set_pb_message("downloading dkg contract...");
        download_cosmwasm_contract(&out_dir, &ci_build_branch, contract_build_names::DKG).await?;

        ctx.set_pb_prefix("[5/7]");
        ctx.set_pb_message("downloading cw4-group contract...");
        download_cosmwasm_contract(&out_dir, &ci_build_branch, contract_build_names::GROUP).await?;

        ctx.set_pb_prefix("[6/7]");
        ctx.set_pb_message("downloading cw3-multisig contract...");
        download_cosmwasm_contract(&out_dir, &ci_build_branch, contract_build_names::MULTISIG)
            .await?;

        ctx.set_pb_prefix("[7/7]");
        ctx.set_pb_message("downloading performance contract...");
        download_cosmwasm_contract(
            &out_dir,
            &ci_build_branch,
            contract_build_names::PERFORMANCE,
        )
        .await?;

        Ok(())
    }

    async fn build_contract(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
        contract_relative_path: &str,
    ) -> anyhow::Result<()> {
        let code_volume = format!("{}:/code", ctx.data.monorepo_root.to_string_lossy());
        let target_volume = format!("type=volume,source={CONTRACTS_CACHE_VOLUME},target=/target");
        let registry_volume = format!(
            "type=volume,source={CARGO_REGISTRY_CACHE_VOLUME},target=/usr/local/cargo/registry"
        );

        let mut args = vec![
            "run",
            "--rm",
            "-v",
            &code_volume,
            "--mount",
            &target_volume,
            "--mount",
            &registry_volume,
        ];

        if ctx.data.reproducible_builds {
            args.push("--platform");
            args.push("linux/amd64");
            args.push("-e");
            args.push("CARGO_BUILD_INCREMENTAL=false");
            args.push("-e");
            args.push(r#"RUSTFLAGS="-C target-cpu=generic -C debuginfo=0""#);
            args.push("-e");
            args.push("SOURCE_DATE_EPOCH=1");
        }

        // the final bit with the actual image and args, e.g. cosmwasm/optimizer:0.17.0 contracts/performance
        args.push(&ctx.data.cosmwasm_optimizer_image);
        args.push(contract_relative_path);

        ctx.execute_cmd_with_exit_status("docker", args).await?;

        Ok(())
    }

    async fn build_cosmwasm_contracts(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step(
            "building cosmwasm contracts... this might take up to 20min if using reproducible builds...",
            "🏗️",
        );

        ctx.set_pb_prefix("[1/9]");
        ctx.set_pb_message("cleaning up build volumes...");
        ctx.exec_fallible_cmd_with_exit_status("docker", ["volume", "rm", CONTRACTS_CACHE_VOLUME])
            .await?;
        ctx.exec_fallible_cmd_with_exit_status(
            "docker",
            ["volume", "rm", CARGO_REGISTRY_CACHE_VOLUME],
        )
        .await?;

        ctx.set_pb_prefix("[2/9]");
        ctx.set_pb_message("building the mixnet contract...");
        self.build_contract(ctx, "contracts/mixnet").await?;

        ctx.set_pb_prefix("[3/9]");
        ctx.set_pb_message("building the vesting contract...");
        self.build_contract(ctx, "contracts/vesting").await?;

        ctx.set_pb_prefix("[4/9]");
        ctx.set_pb_message("building the ecash contract...");
        self.build_contract(ctx, "contracts/ecash").await?;

        ctx.set_pb_prefix("[5/9]");
        ctx.set_pb_message("building the dkg contract...");
        self.build_contract(ctx, "contracts/coconut-dkg").await?;

        ctx.set_pb_prefix("[6/9]");
        ctx.set_pb_message("building the cw4-group contract...");
        self.build_contract(ctx, "contracts/multisig/cw4-group")
            .await?;

        ctx.set_pb_prefix("[7/9]");
        ctx.set_pb_message("building the cw3-multisig contract...");
        self.build_contract(ctx, "contracts/multisig/cw3-flex-multisig")
            .await?;

        ctx.set_pb_prefix("[8/9]");
        ctx.set_pb_message("building the performance contract...");
        self.build_contract(ctx, "contracts/performance").await?;

        ctx.set_pb_prefix("[9/9]");
        ctx.set_pb_message("moving output .wasm files to the target directory");

        let out_dir = self.contracts_wasm_dir(ctx);
        fs::create_dir_all(&out_dir)?;

        let artifacts_dir = ctx.data.monorepo_root.join("artifacts");
        for dir_entry in artifacts_dir.read_dir()? {
            let entry = dir_entry?;
            let build_path = entry.path();
            let Some(extension) = build_path.extension() else {
                continue;
            };
            let Some(filename) = build_path.file_name() else {
                continue;
            };
            let out = out_dir.join(filename);
            if extension.to_string_lossy() == "wasm" {
                debug!("moving {} to {}", build_path.display(), out.display());
                std::fs::rename(&build_path, &out)?;

                // copy it to cache as well
                let cache_path = self
                    .storage
                    .data_cache()
                    .contracts_directory()
                    .join(filename);
                fs::copy(out, cache_path).context("failed to move built contract to the cache")?;
            }
        }

        Ok(())
    }

    /// Check if every expected .wasm file exists in the specified directory
    fn check_contracts_built(&self, ctx: &LocalnetContext<ContractsSetup>) -> bool {
        // check cache if possible
        if ctx.data.allow_cached_build {
            let cached_exists = nym_cosmwasm_contract_names()
                .iter()
                .all(|filename| self.storage.data_cache().cached_contract_exists(filename));
            if cached_exists {
                return true;
            }
        }

        // fallback to default
        nym_cosmwasm_contract_names().iter().all(|filename| {
            let path = self.contracts_wasm_dir(ctx).join(filename);
            path.exists()
        })
    }

    fn set_contracts_build_paths(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        if ctx.data.allow_cached_build
            && ctx
                .data
                .contracts
                .discover_paths(self.storage.data_cache().contracts_directory())
                .is_ok()
        {
            info!("using cached contracts");
            return Ok(());
        }

        ctx.data
            .contracts
            .discover_paths(self.contracts_wasm_dir(ctx))
    }

    // SAFETY: we have an entry for each contract
    #[allow(clippy::unwrap_used)]
    async fn upload_contracts(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("uploading contracts...", "🚚");

        let total = NymContractsBeingInitialised::COUNT as u64;

        let mut upload_results = VecDeque::new();
        for (progress, contract) in ctx.data.contracts.all().into_iter().enumerate() {
            ctx.set_pb_prefix(format!("[{}/{total}]", progress + 1));
            let name = &contract.name;
            ctx.set_pb_message(format!("uploading {name} contract..."));

            let upload_res = self.upload_contract(ctx, &contract.wasm_path()?).await?;
            ctx.println(format!(
                "\t{name} contract uploaded with code: {}. tx: {}",
                upload_res.code_id, upload_res.transaction_hash
            ));
            upload_results.push_back(upload_res.into());
        }
        // we have to assign this in separate loop due to borrow checker rules
        // (iterating for the second time was the simplest workaround)
        for contract in ctx.data.contracts.all_mut() {
            contract.upload_info = Some(upload_results.pop_front().unwrap())
        }

        ctx.println("\t✅ uploaded all the contracts!");

        Ok(())
    }

    async fn prepare_contract_accounts(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step(
            "preparing contract accounts and sending initial tokens...",
            "💸",
        );

        // generate contract admins
        let mut new_accounts = Vec::new();
        for contract in ctx.data.contracts.all_mut() {
            let admin = Account::new();
            debug!(
                "\t{} is going to be admin for the {} contract",
                admin.address, contract.name
            );
            new_accounts.push(admin.address());
            contract.admin = Some(admin);
        }

        // apart from contract admins, we need to send tokens to the mixnet rewarder
        // and the network monitor
        for nm in &ctx.data.auxiliary_accounts.network_monitor {
            new_accounts.push(nm.address())
        }
        new_accounts.push(ctx.data.auxiliary_accounts.mixnet_rewarder.address());

        let receivers = new_accounts
            .into_iter()
            .map(|addr| (addr, ctx.unyms(1000_000000)))
            .collect::<Vec<_>>();

        let signing_client = self.master_signing_client()?;
        let send_fut = signing_client.send_multiple(receivers, "localnet token seeding", None);
        let res = ctx.async_with_progress(send_fut).await?;
        ctx.println(format!(
            "\t✅ sent tokens in transaction: {} (height {})",
            res.hash, res.height
        ));

        Ok(())
    }

    async fn instantiate_contract<T>(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
        contract_name: &'static str,
        init_msg: &T,
    ) -> anyhow::Result<()>
    where
        T: ?Sized + Serialize + Sync,
    {
        let contract = ctx.data.contracts.by_filename(contract_name)?;
        let signer = self.contract_signer(contract)?;

        let code_id = contract.code_id()?;
        let admin = contract.admin_address()?;
        let name = &contract.name;
        // send tx
        let init_fut = signer.instantiate(
            code_id,
            init_msg,
            format!("{name} contract"),
            "localnet contract init",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address} in tx: {}",
            res.transaction_hash
        ));

        // update init info
        let contract_mut = ctx.data.contracts.by_filename_mut(contract_name)?;
        contract_mut.init_info = Some(res.into());

        Ok(())
    }

    async fn migrate_contract<T>(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
        contract_name: &'static str,
        migrate_msg: &T,
    ) -> anyhow::Result<()>
    where
        T: ?Sized + Serialize + Sync,
    {
        let contract = ctx.data.contracts.by_filename(contract_name)?;
        let code_id = contract.code_id()?;
        let address = contract.address()?;
        let admin = contract.admin()?;
        let signer = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic_and_network_details(
            self.localnet_details.localhost_rpc_endpoint()?.as_str(),
            self.localnet_details.nym_network_details()?,
            admin.mnemonic.clone(),
        )?;

        let name = &contract.name;
        // send tx
        let init_fut = signer.migrate(
            address,
            code_id,
            migrate_msg,
            "localnet contract migrate",
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        ctx.println(format!(
            "\t{name} contract migrated in tx: {}",
            res.transaction_hash
        ));

        // update migrate info
        let contract_mut = ctx.data.contracts.by_filename_mut(contract_name)?;
        contract_mut.migrate_info = Some(res.into());

        Ok(())
    }

    // TODO: there are certainly multiple testing scenario where custom contract configuration would be desirable,
    // for example shorter epochs, shorter key rotation, smaller active set, etc.
    // however, for the time being, this is out of scope
    async fn instantiate_contracts(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("instantiating all the contracts...", "💽");

        // ===== MIXNET =====
        ctx.set_pb_prefix("[1/7]");
        ctx.set_pb_message("instantiating the mixnet contract...");

        let init_msg = self.mixnet_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::MIXNET, &init_msg)
            .await?;
        // ===== MIXNET =====

        // ===== VESTING =====
        ctx.set_pb_prefix("[2/7]");
        ctx.set_pb_message("instantiating the vesting contract...");
        let init_msg = self.vesting_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::VESTING, &init_msg)
            .await?;
        // ===== VESTING =====

        // ===== GROUP =====
        ctx.set_pb_prefix("[3/7]");
        ctx.set_pb_message("instantiating the group contract...");
        let init_msg = self.cw4_group_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::GROUP, &init_msg)
            .await?;
        // ===== GROUP =====

        // ===== MULTISIG =====
        ctx.set_pb_prefix("[4/7]");
        ctx.set_pb_message("instantiating the multisig contract...");
        let init_msg = self.cw3_multisig_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::MULTISIG, &init_msg)
            .await?;
        // ===== MULTISIG =====

        // ===== DKG =====
        ctx.set_pb_prefix("[5/7]");
        ctx.set_pb_message("instantiating the dkg contract...");
        let init_msg = self.dkg_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::DKG, &init_msg)
            .await?;
        // ===== DKG =====

        // ===== ECASH =====
        ctx.set_pb_prefix("[6/7]");
        ctx.set_pb_message("instantiating the ecash contract...");
        let init_msg = self.ecash_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::ECASH, &init_msg)
            .await?;
        // ===== ECASH =====

        // ===== PERFORMANCE =====
        ctx.set_pb_prefix("[7/7]");
        ctx.set_pb_message("instantiating the performance contract...");
        let init_msg = self.performance_init_message(ctx)?;
        self.instantiate_contract(ctx, contract_build_names::PERFORMANCE, &init_msg)
            .await?;
        // ===== PERFORMANCE =====

        Ok(())
    }

    async fn perform_required_migrations(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("performing final migrations and contract cleanup...", "🧹");

        // ===== MIXNET =====
        ctx.set_pb_prefix("[1/2]");
        ctx.set_pb_message("migrating the mixnet contract (fixing up vesting contract address)...");
        let migrate_msg = self.mixnet_migrate_message(ctx)?;
        self.migrate_contract(ctx, contract_build_names::MIXNET, &migrate_msg)
            .await?;
        // ===== MIXNET =====

        // ===== MULTISIG =====
        ctx.set_pb_prefix("[2/2]");
        ctx.set_pb_message(
            "migrating the multisig contract (fixing up ecash and dkg contract addresses)...",
        );
        let migrate_msg = self.multisig_migrate_message(ctx)?;
        self.migrate_contract(ctx, contract_build_names::MULTISIG, &migrate_msg)
            .await?;
        // ===== MULTISIG =====

        ctx.println("\t✅ performed all the needed migrations!");

        Ok(())
    }

    // the purpose of this function is two-fold:
    // 1. figure out how old are the contracts
    // 2. (more important): implicitly verify they have correct structure, i.e. at the very least
    // actually DO store the build information
    async fn validate_build_information(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("inspecting contracts build information...", "🔍");

        let client = self.rpc_query_client()?;

        let now = OffsetDateTime::now_utc();
        for contract in ctx.data.contracts.all() {
            let build_info_fut = client.try_get_contract_build_information(contract.address()?);
            let name = &contract.name;
            let build_info = ctx
                .async_with_progress(build_info_fut)
                .await
                .context(format!(
                    "missing contract build information for {name} contract",
                ))?;
            let built_time = OffsetDateTime::parse(&build_info.build_timestamp, &Rfc3339)?;
            let age = now - built_time;
            let age_secs = Duration::from_secs(age.whole_seconds() as u64);
            let age_human = humantime::format_duration(age_secs); // no need for ns precision in logs
            let emoji = if age > time::Duration::days(30) {
                "☠️️"
            } else if age > time::Duration::days(7) {
                "❗️"
            } else if age > time::Duration::days(1) {
                "️️⚠️"
            } else {
                "ℹ️"
            };
            ctx.println_with_emoji(
                format!("the {name} contract has been built {age_human} ago",),
                emoji,
            );
        }

        Ok(())
    }

    async fn finalize_contracts_setup(
        &mut self,
        mut ctx: LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("persisting cosmwasm contract details", "📝");

        // update state
        self.localnet_details
            .set_auxiliary_accounts(ctx.data.auxiliary_accounts)
            .set_contracts(ctx.data.contracts.into_built_contracts()?);

        let localnet_name = &self.localnet_details.human_name;
        self.storage
            .orchestrator()
            .save_auxiliary_accounts(localnet_name, self.localnet_details.auxiliary_accounts()?)
            .await?;
        self.storage
            .orchestrator()
            .save_localnet_contracts(localnet_name, self.localnet_details.contracts()?)
            .await?;
        self.state = LocalnetState::DeployedNymContracts;

        Ok(())
    }

    pub(crate) async fn wait_for_first_block(
        &self,
        ctx: &mut LocalnetContext<ContractsSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("waiting for the chain to produce its first block...", "⏳");

        let client = self.rpc_query_client()?;
        tokio::time::timeout(Duration::from_secs(10), async move {
            loop {
                if let Ok(height) = client.get_current_block_height().await {
                    if height.value() >= 2 {
                        return Ok::<_, anyhow::Error>(());
                    }
                }
            }
        })
        .await??;

        Ok(())
    }

    pub(crate) async fn initialise_contracts(&mut self, config: Config) -> anyhow::Result<()> {
        // 0. establish initial nyxd details

        let setup = ContractsSetup::new(config)?;
        let mut ctx = LocalnetContext::new(setup, 9, "\nsetting up cosmwasm contracts");

        // 1.1 wait for the chain to produce its first block
        self.wait_for_first_block(&mut ctx).await?;

        // 1.2 check rpc connection and master account existence
        self.verify_master_account(&ctx).await?;

        // 2. if requested, attempt to download the contracts
        if ctx.data.ci_build_branch.is_some() {
            self.download_cosmwasm_contracts(&mut ctx).await?;
        } else {
            ctx.skip_steps(1);
        }

        // 3.1 check if contracts have already been built
        if self.check_contracts_built(&ctx) {
            info!("required contracts have already been built - skipping the step");
            ctx.skip_steps(1);
        } else {
            // 3.2. create .wasm files
            self.build_cosmwasm_contracts(&mut ctx).await?;
        }

        // 4.1 update internal metadata (internally figure out paths to all .wasm files)
        self.set_contracts_build_paths(&mut ctx)?;

        // 4.2 upload the contracts
        self.upload_contracts(&mut ctx).await?;

        // 5. create mnemonics + transfer tokens
        self.prepare_contract_accounts(&mut ctx).await?;

        // 6 init the contracts
        self.instantiate_contracts(&mut ctx).await?;

        // 7. perform state migrations to fix up initial states
        self.perform_required_migrations(&mut ctx).await?;

        // 8. verify build info
        self.validate_build_information(&mut ctx).await?;

        // 9. persist all information
        self.finalize_contracts_setup(ctx).await?;
        Ok(())
    }
}
