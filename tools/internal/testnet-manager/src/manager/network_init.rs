// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{async_with_progress, ProgressCtx, ProgressTracker};
use crate::manager::contract::Account;
use crate::manager::network::Network;
use crate::manager::NetworkManager;
use console::style;
use cw_utils::Threshold;
use indicatif::HumanDuration;
use nym_coconut_dkg_common::types::TimeConfiguration;
use nym_config::defaults::NymNetworkDetails;
use nym_mixnet_contract_common::{Decimal, InitialRewardingParams, Percent};
use nym_validator_client::nyxd::cosmwasm_client::types::InstantiateOptions;
use nym_validator_client::nyxd::Config;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::ops::Deref;
use std::path::Path;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use url::Url;

struct InitCtx {
    progress: ProgressTracker,
    network: Network,
    admin: DirectSigningHttpRpcNyxdClient,
}

impl InitCtx {
    fn dummy_client_config() -> Result<Config, NetworkManagerError> {
        // ASSUMPTION: same chain details like prefix, denoms, etc. as mainnet
        let mainnet = NymNetworkDetails::new_mainnet();
        let network_details = NymNetworkDetails {
            chain_details: mainnet.chain_details,
            network_name: "foomp".to_string(), // does this matter?
            endpoints: vec![],
            contracts: Default::default(),
            explorer_api: None,
            nym_vpn_api_url: None,
        };
        Ok(Config::try_from_nym_network_details(&network_details)?)
    }

    fn new(
        network_name: String,
        admin_mnemonic: bip39::Mnemonic,
        rpc_endpoint: &Url,
    ) -> Result<Self, NetworkManagerError> {
        let admin = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            Self::dummy_client_config()?,
            rpc_endpoint.as_str(),
            admin_mnemonic,
        )?;

        let progress = ProgressTracker::new(format!(
            "\n🚀 setting up new testnet '{network_name}' over {rpc_endpoint}",
        ));

        Ok(InitCtx {
            progress,
            network: Network {
                name: network_name,
                rpc_endpoint: rpc_endpoint.clone(),
                created_at: OffsetDateTime::now_utc(),
                contracts: Default::default(),
                auxiliary_addresses: Default::default(),
            },
            admin,
        })
    }

    fn mixnet_signing_client(&self) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            Self::dummy_client_config()?,
            self.network.rpc_endpoint.as_str(),
            self.network.contracts.mixnet.admin()?.mnemonic.clone(),
        )?)
    }

    fn multisig_signing_client(
        &self,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            Self::dummy_client_config()?,
            self.network.rpc_endpoint.as_str(),
            self.network
                .contracts
                .cw3_multisig
                .admin()?
                .mnemonic
                .clone(),
        )?)
    }
}

impl ProgressCtx for InitCtx {
    fn progress_tracker(&self) -> &ProgressTracker {
        &self.progress
    }
}

impl NetworkManager {
    fn mixnet_migrate_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_mixnet_contract_common::MigrateMsg, NetworkManagerError> {
        Ok(nym_mixnet_contract_common::MigrateMsg {
            vesting_contract_address: Some(ctx.network.contracts.vesting.address()?.to_string()),
        })
    }

    fn multisig_migrate_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_multisig_contract_common::msg::MigrateMsg, NetworkManagerError> {
        Ok(nym_multisig_contract_common::msg::MigrateMsg {
            coconut_bandwidth_address: ctx.network.contracts.ecash.address()?.to_string(),
            coconut_dkg_address: ctx.network.contracts.dkg.address()?.to_string(),
        })
    }

    fn mixnet_init_message(
        &self,
        ctx: &InitCtx,
        custom_epoch_duration: Option<Duration>,
    ) -> Result<nym_mixnet_contract_common::InstantiateMsg, NetworkManagerError> {
        Ok(nym_mixnet_contract_common::InstantiateMsg {
            rewarding_validator_address: ctx
                .network
                .auxiliary_addresses
                .mixnet_rewarder
                .address
                .to_string(),
            // PLACEHOLDER \/
            vesting_contract_address: ctx
                .network
                .auxiliary_addresses
                .mixnet_rewarder
                .address
                .to_string(),
            // PLACEHOLDER /\
            rewarding_denom: ctx.admin.mix_coin(0).denom,
            epochs_in_interval: 720,
            epoch_duration: custom_epoch_duration.unwrap_or(Duration::from_secs(60 * 60)),
            initial_rewarding_params: InitialRewardingParams {
                initial_reward_pool: Decimal::from_atomics(250_000_000_000_000u128, 0).unwrap(),
                initial_staking_supply: Decimal::from_atomics(100_000_000_000_000u128, 0).unwrap(),
                staking_supply_scale_factor: Percent::from_percentage_value(50).unwrap(),
                sybil_resistance: Percent::from_percentage_value(30).unwrap(),
                active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                interval_pool_emission: Percent::from_percentage_value(2).unwrap(),
                rewarded_set_size: 240,
                active_set_size: 240,
            },
            profit_margin: Default::default(),
            interval_operating_cost: Default::default(),
        })
    }

    fn vesting_init_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_vesting_contract_common::InitMsg, NetworkManagerError> {
        Ok(nym_vesting_contract_common::InitMsg {
            mixnet_contract_address: ctx.network.contracts.mixnet.address()?.to_string(),
            mix_denom: ctx.admin.mix_coin(0).denom,
        })
    }

    fn dkg_init_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_coconut_dkg_common::msg::InstantiateMsg, NetworkManagerError> {
        Ok(nym_coconut_dkg_common::msg::InstantiateMsg {
            group_addr: ctx.network.contracts.cw4_group.address()?.to_string(),
            multisig_addr: ctx.network.contracts.cw3_multisig.address()?.to_string(),
            time_configuration: Some(TimeConfiguration {
                public_key_submission_time_secs: 3600,
                dealing_exchange_time_secs: 3600,
                verification_key_submission_time_secs: 3600,
                verification_key_validation_time_secs: 3600,
                verification_key_finalization_time_secs: 3600,
                in_progress_time_secs: 10000000000,
            }),
            mix_denom: ctx.admin.mix_coin(0).denom,
            key_size: 5,
        })
    }

    fn ecash_init_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_ecash_contract_common::msg::InstantiateMsg, NetworkManagerError> {
        Ok(nym_ecash_contract_common::msg::InstantiateMsg {
            holding_account: ctx
                .network
                .auxiliary_addresses
                .ecash_holding_account
                .address
                .to_string(),
            multisig_addr: ctx.network.contracts.cw3_multisig.address()?.to_string(),
            group_addr: ctx.network.contracts.cw4_group.address()?.to_string(),
            deposit_amount: ctx.admin.mix_coin(75_000_000).into(),
        })
    }

    fn cw3_multisig_init_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_multisig_contract_common::msg::InstantiateMsg, NetworkManagerError> {
        Ok(nym_multisig_contract_common::msg::InstantiateMsg {
            group_addr: ctx.network.contracts.cw4_group.address()?.to_string(),

            // PLACEHOLDER \/
            coconut_bandwidth_contract_address: ctx
                .network
                .contracts
                .cw4_group
                .address()?
                .to_string(),
            coconut_dkg_contract_address: ctx.network.contracts.cw4_group.address()?.to_string(),
            // PLACEHOLDER /\
            threshold: Threshold::AbsolutePercentage {
                percentage: "0.67".parse().unwrap(),
            },
            max_voting_period: cw_utils::Duration::Time(3600),
            executor: None,
            proposal_deposit: None,
        })
    }

    fn cw4_group_init_message(
        &self,
        ctx: &InitCtx,
    ) -> Result<nym_group_contract_common::msg::InstantiateMsg, NetworkManagerError> {
        Ok(nym_group_contract_common::msg::InstantiateMsg {
            admin: Some(
                ctx.network
                    .contracts
                    .cw4_group
                    .admin()?
                    .address()
                    .to_string(),
            ),
            // TODO: prepopulate
            members: vec![],
        })
    }

    fn find_contracts<P: AsRef<Path>>(
        &self,
        ctx: &mut InitCtx,
        base_dir: P,
    ) -> Result<(), NetworkManagerError> {
        ctx.network.contracts.discover_paths(base_dir)?;

        ctx.println(format!(
            "🔍 {}Locating .wasm files...",
            style("[1/8]").bold().dim()
        ));
        ctx.println(format!(
            "\tdiscovered mixnet contract at '{}'",
            ctx.network.contracts.mixnet.wasm_path()?.display()
        ));
        ctx.println(format!(
            "\tdiscovered vesting contract at '{}'",
            ctx.network.contracts.vesting.wasm_path()?.display()
        ));
        ctx.println(format!(
            "\tdiscovered ecash contract at '{}'",
            ctx.network.contracts.ecash.wasm_path()?.display()
        ));
        ctx.println(format!(
            "\tdiscovered cw4_group contract at '{}'",
            ctx.network.contracts.cw4_group.wasm_path()?.display()
        ));
        ctx.println(format!(
            "\tdiscovered cw3_multisig contract at '{}'",
            ctx.network.contracts.cw3_multisig.wasm_path()?.display()
        ));
        ctx.println(format!(
            "\tdiscovered dkg contract at '{}'",
            ctx.network.contracts.dkg.wasm_path()?.display()
        ));

        ctx.println("\t✅ found all the contracts!");

        Ok(())
    }

    async fn upload_contracts(&self, ctx: &mut InitCtx) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🚚 {}Uploading contracts...",
            style("[2/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;
        let pb = &ctx.progress.progress_bar;

        for (progress, contract) in ctx
            .network
            .contracts
            .fake_iter_mut()
            .into_iter()
            .enumerate()
        {
            pb.set_prefix(format!("[{}/{total}]", progress + 1));
            let name = &contract.name;
            pb.set_message(format!("uploading {name} contract..."));
            let upload_res = self
                .upload_contract(
                    &ctx.admin,
                    &ctx.progress.progress_bar,
                    &contract.wasm_path()?,
                )
                .await?;
            pb.println(format!(
                "\t{name} contract uploaded with code: {}",
                upload_res.code_id
            ));
            contract.upload_info = Some(upload_res.into());
        }

        ctx.println("\t✅ uploaded all the contracts!");

        Ok(())
    }

    fn create_contract_admins_mnemonics(
        &self,
        ctx: &mut InitCtx,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "📝 {}Generating admin mnemonics...",
            style("[3/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;
        let pb = &ctx.progress.progress_bar;
        for (progress, contract) in ctx
            .network
            .contracts
            .fake_iter_mut()
            .into_iter()
            .enumerate()
        {
            pb.set_prefix(format!("[{}/{total}]", progress + 1));
            let name = &contract.name;
            pb.set_message(format!("generating admin mnemonic for {name} contract..."));
            let admin = Account::new();
            pb.println(format!(
                "\t{} is going to be admin for the {name} contract",
                admin.address
            ));
            contract.admin = Some(admin)
        }

        ctx.println("\t✅ generated all admin mnemonics!");

        Ok(())
    }

    async fn transfer_admin_tokens(&self, ctx: &InitCtx) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "💸 {}Transferring tokens to the admin accounts...",
            style("[4/8]").bold().dim()
        ));

        let mut receivers = Vec::new();
        for contract in ctx.network.contracts.fake_iter() {
            // send 10nym to the admin
            receivers.push((contract.admin()?.address(), ctx.admin.mix_coins(10_000000)))
        }

        // also send them to the rewarder
        receivers.push((
            ctx.network.auxiliary_addresses.mixnet_rewarder.address(),
            ctx.admin.mix_coins(10_000000),
        ));

        ctx.set_pb_message("attempting to send admin tokens...");

        let send_future =
            ctx.admin
                .send_multiple(receivers, "admin token transfer from testnet-manager", None);
        let res = ctx.async_with_progress(send_future).await?;

        ctx.println(format!(
            "\t✅ sent tokens in transaction: {} (height {})",
            res.hash, res.height
        ));

        Ok(())
    }

    async fn instantiate_contracts(
        &self,
        ctx: &mut InitCtx,
        custom_epoch_duration: Option<Duration>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "💽 {}Instantiating all the contracts...",
            style("[5/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;

        // mixnet
        ctx.set_pb_prefix(format!("[1/{total}]"));
        let name = &ctx.network.contracts.mixnet.name;
        let code_id = ctx.network.contracts.mixnet.upload_info()?.code_id;
        let admin = ctx.network.contracts.mixnet.admin()?.address.clone();
        ctx.set_pb_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.mixnet_init_message(ctx, custom_epoch_duration)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from testnet-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.mixnet.init_info = Some(res.into());

        // vesting
        ctx.set_pb_prefix(format!("[2/{total}]"));
        let name = &ctx.network.contracts.vesting.name;
        let code_id = ctx.network.contracts.vesting.upload_info()?.code_id;
        let admin = ctx.network.contracts.vesting.admin()?.address.clone();
        ctx.set_pb_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.vesting_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from testnet-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.vesting.init_info = Some(res.into());

        // group
        ctx.set_pb_prefix(format!("[3/{total}]"));
        let name = &ctx.network.contracts.cw4_group.name;
        let code_id = ctx.network.contracts.cw4_group.upload_info()?.code_id;
        let admin = ctx.network.contracts.cw4_group.admin()?.address.clone();
        ctx.set_pb_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.cw4_group_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from testnet-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.cw4_group.init_info = Some(res.into());

        // multisig
        ctx.set_pb_prefix(format!("[4/{total}]"));
        let name = &ctx.network.contracts.cw3_multisig.name;
        let code_id = ctx.network.contracts.cw3_multisig.upload_info()?.code_id;
        let admin = ctx.network.contracts.cw3_multisig.admin()?.address.clone();
        ctx.set_pb_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.cw3_multisig_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from testnet-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.cw3_multisig.init_info = Some(res.into());

        // dkg
        ctx.set_pb_prefix(format!("[5/{total}]"));
        let name = &ctx.network.contracts.dkg.name;
        let code_id = ctx.network.contracts.dkg.upload_info()?.code_id;
        let admin = ctx.network.contracts.dkg.admin()?.address.clone();
        ctx.set_pb_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.dkg_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from testnet-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.dkg.init_info = Some(res.into());

        // ecash
        ctx.set_pb_prefix(format!("[6/{total}]"));
        let name = &ctx.network.contracts.ecash.name;
        let code_id = ctx.network.contracts.ecash.upload_info()?.code_id;
        let admin = ctx.network.contracts.ecash.admin()?.address.clone();
        ctx.set_pb_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.ecash_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from testnet-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = ctx.async_with_progress(init_fut).await?;
        let address = &res.contract_address;
        ctx.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.ecash.init_info = Some(res.into());

        ctx.println("\t✅ instantiated all the contracts!");

        Ok(())
    }

    async fn perform_final_migrations(&self, ctx: &mut InitCtx) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🧹 {}Performing final migrations and contract cleanup...",
            style("[6/8]").bold().dim()
        ));

        // migrate mixnet
        ctx.set_pb_prefix("[1/2]");
        let name = &ctx.network.contracts.mixnet.name;
        let code_id = ctx.network.contracts.mixnet.upload_info()?.code_id;
        let address = ctx.network.contracts.mixnet.address()?;
        ctx.set_pb_message(format!("attempting to migrate {name} contract..."));
        let migrate_msg = self.mixnet_migrate_message(ctx)?;
        let client = ctx.mixnet_signing_client()?;
        let migrate_fut = client.migrate(
            address,
            code_id,
            &migrate_msg,
            "contract migration from testnet-manager",
            None,
        );
        let migrate_res = ctx.async_with_progress(migrate_fut).await?;
        ctx.network.contracts.mixnet.migrate_info = Some(migrate_res.into());
        ctx.println(format!("\t{name} contract has been migrated"));

        // migrate multisig
        ctx.set_pb_prefix("[2/2]");
        let name = &ctx.network.contracts.cw3_multisig.name;
        let code_id = ctx.network.contracts.cw3_multisig.upload_info()?.code_id;
        let address = ctx.network.contracts.cw3_multisig.address()?;
        ctx.set_pb_message(format!("attempting to migrate {name} contract..."));
        let migrate_msg = self.multisig_migrate_message(ctx)?;
        let client = ctx.multisig_signing_client()?;
        let migrate_fut = client.migrate(
            address,
            code_id,
            &migrate_msg,
            "contract migration from testnet-manager",
            None,
        );
        let migrate_res = ctx.async_with_progress(migrate_fut).await?;
        ctx.network.contracts.cw3_multisig.migrate_info = Some(migrate_res.into());
        ctx.println(format!("\t{name} contract has been migrated"));

        ctx.println("\t✅ performed all the needed migrations!");

        Ok(())
    }

    async fn get_build_info(&self, ctx: &mut InitCtx) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🏗️ {}Obtaining contracts build information",
            style("[7/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;

        let pb = &ctx.progress.progress_bar;
        for (progress, contract) in ctx
            .network
            .contracts
            .fake_iter_mut()
            .into_iter()
            .enumerate()
        {
            pb.set_prefix(format!("[{}/{total}]", progress + 1));
            let name = &contract.name;
            let address = contract.address()?;
            pb.set_message(format!("querying {name} contract..."));
            let build_info_fut = ctx.admin.try_get_contract_build_information(address);
            let build_info = async_with_progress(build_info_fut, &ctx.progress.progress_bar)
                .await
                .ok_or_else(|| NetworkManagerError::MissingBuildInfo {
                    name: name.to_string(),
                })?;

            let now = OffsetDateTime::now_utc();
            // SAFETY: all the information saved in our contracts should be well-formed
            let commit_timestamp = OffsetDateTime::parse(&build_info.commit_timestamp, &Rfc3339)
                .expect("malformed commit timestamp");

            let age = now - commit_timestamp;

            pb.println(format!(
                "\t{name} contract was built from branch: {} (sha: {}); age: {}",
                build_info.commit_branch,
                build_info.commit_sha,
                HumanDuration(age.unsigned_abs())
            ));

            if age > time::Duration::days(30) {
                pb.println(format!(
                    "\t\t️☠️️ {}",
                    style("this commit is ANCIENT - please double check if this is intended")
                        .bold()
                        .red()
                ))
            } else if age > time::Duration::days(7) {
                pb.println(format!(
                    "\t\t️❗️ {}",
                    style("this commit is rather old - please double check if this is intended")
                        .bold()
                        .red()
                ))
            } else if age > time::Duration::days(1) {
                pb.println(format!(
                    "\t\t️️⚠️ {}",
                    style("this commit seems outdated - please double check if this is intended")
                        .bold()
                        .yellow()
                ))
            }

            contract.build_info = Some(build_info);
        }

        ctx.println("\t✅ updated all contract metadata!");

        Ok(())
    }

    async fn persist_network_in_database(&self, ctx: &InitCtx) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "📦 {}Storing all the results in the database",
            style("[8/8]").bold().dim()
        ));

        ctx.set_pb_message("attempting to persist network data...");
        let save_future = self.storage.persist_network(&ctx.network);
        ctx.async_with_progress(save_future).await?;

        ctx.println("\t✅ the network information got persisted in the database for future use");

        Ok(())
    }

    pub(crate) async fn initialise_new_network<P: AsRef<Path>>(
        &self,
        contracts: P,
        network_name: Option<String>,
        custom_epoch_duration: Option<Duration>,
    ) -> Result<Network, NetworkManagerError> {
        let network_name = self.get_network_name(network_name);
        let mut ctx = InitCtx::new(network_name, self.admin.deref().clone(), &self.rpc_endpoint)?;

        self.find_contracts(&mut ctx, contracts)?;
        self.upload_contracts(&mut ctx).await?;
        self.create_contract_admins_mnemonics(&mut ctx)?;
        self.transfer_admin_tokens(&ctx).await?;
        self.instantiate_contracts(&mut ctx, custom_epoch_duration)
            .await?;
        self.perform_final_migrations(&mut ctx).await?;
        self.get_build_info(&mut ctx).await?;
        self.persist_network_in_database(&ctx).await?;

        Ok(ctx.network.clone())
    }
}
