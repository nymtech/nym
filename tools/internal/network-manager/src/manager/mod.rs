// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::error::NetworkManagerError;
use crate::helpers::async_with_progress;
use crate::manager::contract::Account;
use crate::manager::network::{LoadedNetwork, Network};
use crate::manager::storage::NetworkManagerStorage;
use bip39::rand::prelude::SliceRandom;
use bip39::rand::thread_rng;
use console::style;
use cw_utils::Threshold;
use indicatif::{HumanDuration, MultiProgress, ProgressBar};
use nym_coconut_dkg_common::types::TimeConfiguration;
use nym_config::defaults::NymNetworkDetails;
use nym_mixnet_contract_common::{Decimal, InitialRewardingParams, Percent};
use nym_validator_client::nyxd::cosmwasm_client::types::{InstantiateOptions, UploadResult};
use nym_validator_client::nyxd::Config;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;
use std::time::{Duration, Instant};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use url::Url;
use zeroize::Zeroizing;

mod contract;
pub(crate) mod network;
pub(crate) mod storage;

fn wasm_code<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, NetworkManagerError> {
    let path = path.as_ref();
    assert!(path.exists());
    let mut file = std::fs::File::open(path)?;
    let mut data = Vec::new();

    file.read_to_end(&mut data)?;
    Ok(data)
}

struct Ctx {
    network: Network,
    progress_bar: MultiProgress,
    admin: DirectSigningHttpRpcNyxdClient,
}

impl Ctx {
    fn dummy_client_config() -> Result<Config, NetworkManagerError> {
        // ASSUMPTION: same chain details like prefix, denoms, etc. as mainnet
        let mainnet = NymNetworkDetails::new_mainnet();
        let network_details = NymNetworkDetails {
            chain_details: mainnet.chain_details,
            network_name: "foomp".to_string(), // does this matter?
            endpoints: vec![],
            contracts: Default::default(),
            explorer_api: None,
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

        Ok(Ctx {
            network: Network {
                name: network_name,
                rpc_endpoint: rpc_endpoint.clone(),
                created_at: OffsetDateTime::now_utc(),
                contracts: Default::default(),
                auxiliary_addresses: Default::default(),
            },
            progress_bar: MultiProgress::new(),
            admin,
        })
    }

    fn spinner_pb(&self) -> ProgressBar {
        self.progress_bar.add(ProgressBar::new_spinner())
    }

    fn into_network(self) -> Network {
        self.network
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

pub(crate) struct NetworkManager {
    admin: Zeroizing<bip39::Mnemonic>,
    storage: NetworkManagerStorage,
    rpc_endpoint: Url,
}

impl NetworkManager {
    pub(crate) async fn new<P: AsRef<Path>>(
        database_path: P,
        mnemonic: Option<bip39::Mnemonic>,
        rpc_endpoint: Option<Url>,
    ) -> Result<Self, NetworkManagerError> {
        let storage = NetworkManagerStorage::init(database_path).await?;

        let (mnemonic, rpc_endpoint) = if !storage.metadata_set().await? {
            let mnemonic = mnemonic.ok_or(NetworkManagerError::MnemonicNotSet)?;
            let rpc_endpoint = rpc_endpoint.ok_or(NetworkManagerError::RpcEndpointNotSet)?;

            storage
                .set_initial_metadata(&mnemonic, &rpc_endpoint)
                .await?;
            (mnemonic, rpc_endpoint)
        } else {
            let mnemonic = storage
                .get_master_mnemonic()
                .await?
                .ok_or(NetworkManagerError::MnemonicNotSet)?;

            let rpc_endpoint = storage
                .get_rpc_endpoint()
                .await?
                .ok_or(NetworkManagerError::RpcEndpointNotSet)?;

            (mnemonic, rpc_endpoint)
        };

        Ok(NetworkManager {
            admin: Zeroizing::new(mnemonic),
            storage,
            rpc_endpoint,
        })
    }

    fn get_network_name(&self, user_provided: Option<String>) -> String {
        user_provided.unwrap_or_else(|| {
            // a hack to get human-readable words without extra deps : )
            let mut rng = thread_rng();

            let words = bip39::Language::English.word_list();
            let first = words.choose(&mut rng).unwrap();
            let second = words.choose(&mut rng).unwrap();
            format!("{first}-{second}")
        })
    }

    fn mixnet_migrate_message(
        &self,
        ctx: &Ctx,
    ) -> Result<nym_mixnet_contract_common::MigrateMsg, NetworkManagerError> {
        Ok(nym_mixnet_contract_common::MigrateMsg {
            vesting_contract_address: Some(ctx.network.contracts.vesting.address()?.to_string()),
        })
    }

    fn multisig_migrate_message(
        &self,
        ctx: &Ctx,
    ) -> Result<nym_multisig_contract_common::msg::MigrateMsg, NetworkManagerError> {
        Ok(nym_multisig_contract_common::msg::MigrateMsg {
            coconut_bandwidth_address: ctx.network.contracts.ecash.address()?.to_string(),
            coconut_dkg_address: ctx.network.contracts.dkg.address()?.to_string(),
        })
    }

    fn mixnet_init_message(
        &self,
        ctx: &Ctx,
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
            epoch_duration: Duration::from_secs(60 * 60),
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
        })
    }

    fn vesting_init_message(
        &self,
        ctx: &Ctx,
    ) -> Result<nym_vesting_contract_common::InitMsg, NetworkManagerError> {
        Ok(nym_vesting_contract_common::InitMsg {
            mixnet_contract_address: ctx.network.contracts.mixnet.address()?.to_string(),
            mix_denom: ctx.admin.mix_coin(0).denom,
        })
    }

    fn dkg_init_message(
        &self,
        ctx: &Ctx,
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
        ctx: &Ctx,
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
            mix_denom: ctx.admin.mix_coin(0).denom,
        })
    }

    fn cw3_multisig_init_message(
        &self,
        ctx: &Ctx,
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
        ctx: &Ctx,
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
        ctx: &mut Ctx,
        base_dir: P,
    ) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();

        ctx.network.contracts.discover_paths(base_dir)?;

        pb.println(format!(
            "🔍 {}Locating .wasm files...",
            style("[1/8]").bold().dim()
        ));
        pb.println(format!(
            "\tdiscovered mixnet contract at '{}'",
            ctx.network.contracts.mixnet.wasm_path()?.display()
        ));
        pb.println(format!(
            "\tdiscovered vesting contract at '{}'",
            ctx.network.contracts.vesting.wasm_path()?.display()
        ));
        pb.println(format!(
            "\tdiscovered ecash contract at '{}'",
            ctx.network.contracts.ecash.wasm_path()?.display()
        ));
        pb.println(format!(
            "\tdiscovered cw4_group contract at '{}'",
            ctx.network.contracts.cw4_group.wasm_path()?.display()
        ));
        pb.println(format!(
            "\tdiscovered cw3_multisig contract at '{}'",
            ctx.network.contracts.cw3_multisig.wasm_path()?.display()
        ));
        pb.println(format!(
            "\tdiscovered dkg contract at '{}'",
            ctx.network.contracts.dkg.wasm_path()?.display()
        ));

        pb.println("\t✅ found all the contracts!");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    async fn upload_contract<P: AsRef<Path>>(
        &self,
        admin: &DirectSigningHttpRpcNyxdClient,
        pb: &ProgressBar,
        path: P,
    ) -> Result<UploadResult, NetworkManagerError> {
        let wasm = wasm_code(path)?;
        let upload_future = admin.upload(wasm, "contract upload from network-manager", None);

        async_with_progress(upload_future, pb)
            .await
            .map_err(Into::into)
    }

    async fn upload_contracts(&self, ctx: &mut Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();

        pb.println(format!(
            "🚚 {}Uploading contracts...",
            style("[2/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;

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
                .upload_contract(&ctx.admin, &pb, &contract.wasm_path()?)
                .await?;
            pb.println(format!(
                "\t{name} contract uploaded with code: {}",
                upload_res.code_id
            ));
            contract.upload_info = Some(upload_res.into());
        }

        pb.println("\t✅ uploaded all the contracts!");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    fn create_contract_admins_mnemonics(&self, ctx: &mut Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();

        pb.println(format!(
            "📝 {}Generating admin mnemonics...",
            style("[3/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;
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

        pb.println("\t✅ generated all admin mnemonics!");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    async fn transfer_admin_tokens(&self, ctx: &Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();
        pb.println(format!(
            "💸 {}Transferring tokens to the admin accounts...",
            style("[4/8]").bold().dim()
        ));

        let mut receivers = Vec::new();
        for contract in ctx.network.contracts.fake_iter() {
            // send 10nym to the admin
            receivers.push((contract.admin()?.address(), ctx.admin.mix_coins(10_000000)))
        }

        pb.set_message("attempting to send admin tokens...");

        let send_future =
            ctx.admin
                .send_multiple(receivers, "admin token transfer from network-manager", None);
        let res = async_with_progress(send_future, &pb).await?;

        pb.println(format!(
            "\t✅ sent tokens in transaction: {} (height {})",
            res.hash, res.height
        ));
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    async fn instantiate_contracts(&self, ctx: &mut Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();
        pb.println(format!(
            "💽 {}Instantiating all the contracts...",
            style("[5/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;

        // mixnet
        pb.set_prefix(format!("[1/{total}]"));
        let name = &ctx.network.contracts.mixnet.name;
        let code_id = ctx.network.contracts.mixnet.upload_info()?.code_id;
        let admin = ctx.network.contracts.mixnet.admin()?.address.clone();
        pb.set_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.mixnet_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from network-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = async_with_progress(init_fut, &pb).await?;
        let address = &res.contract_address;
        pb.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.mixnet.init_info = Some(res.into());

        // vesting
        pb.set_prefix(format!("[2/{total}]"));
        let name = &ctx.network.contracts.vesting.name;
        let code_id = ctx.network.contracts.vesting.upload_info()?.code_id;
        let admin = ctx.network.contracts.vesting.admin()?.address.clone();
        pb.set_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.vesting_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from network-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = async_with_progress(init_fut, &pb).await?;
        let address = &res.contract_address;
        pb.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.vesting.init_info = Some(res.into());

        // group
        pb.set_prefix(format!("[3/{total}]"));
        let name = &ctx.network.contracts.cw4_group.name;
        let code_id = ctx.network.contracts.cw4_group.upload_info()?.code_id;
        let admin = ctx.network.contracts.cw4_group.admin()?.address.clone();
        pb.set_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.cw4_group_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from network-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = async_with_progress(init_fut, &pb).await?;
        let address = &res.contract_address;
        pb.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.cw4_group.init_info = Some(res.into());

        // multisig
        pb.set_prefix(format!("[4/{total}]"));
        let name = &ctx.network.contracts.cw3_multisig.name;
        let code_id = ctx.network.contracts.cw3_multisig.upload_info()?.code_id;
        let admin = ctx.network.contracts.cw3_multisig.admin()?.address.clone();
        pb.set_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.cw3_multisig_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from network-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = async_with_progress(init_fut, &pb).await?;
        let address = &res.contract_address;
        pb.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.cw3_multisig.init_info = Some(res.into());

        // dkg
        pb.set_prefix(format!("[5/{total}]"));
        let name = &ctx.network.contracts.dkg.name;
        let code_id = ctx.network.contracts.dkg.upload_info()?.code_id;
        let admin = ctx.network.contracts.dkg.admin()?.address.clone();
        pb.set_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.dkg_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from network-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = async_with_progress(init_fut, &pb).await?;
        let address = &res.contract_address;
        pb.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.dkg.init_info = Some(res.into());

        // ecash
        pb.set_prefix(format!("[6/{total}]"));
        let name = &ctx.network.contracts.ecash.name;
        let code_id = ctx.network.contracts.ecash.upload_info()?.code_id;
        let admin = ctx.network.contracts.ecash.admin()?.address.clone();
        pb.set_message(format!("attempting to instantiate {name} contract..."));
        let init_msg = self.ecash_init_message(ctx)?;
        let init_fut = ctx.admin.instantiate(
            code_id,
            &init_msg,
            format!("{name} contract"),
            "contract instantiation from network-manager",
            Some(InstantiateOptions::default().with_admin(admin)),
            None,
        );
        let res = async_with_progress(init_fut, &pb).await?;
        let address = &res.contract_address;
        pb.println(format!(
            "\t{name} contract instantiated with address: {address}",
        ));
        ctx.network.contracts.ecash.init_info = Some(res.into());

        pb.println("\t✅ instantiated all the contracts!");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    async fn perform_final_migrations(&self, ctx: &mut Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();
        pb.println(format!(
            "🧹 {}Performing final migrations and contract cleanup...",
            style("[6/8]").bold().dim()
        ));

        // migrate mixnet
        pb.set_prefix("[1/2]");
        let name = &ctx.network.contracts.mixnet.name;
        let code_id = ctx.network.contracts.mixnet.upload_info()?.code_id;
        let address = ctx.network.contracts.mixnet.address()?;
        pb.set_message(format!("attempting to migrate {name} contract..."));
        let migrate_msg = self.mixnet_migrate_message(ctx)?;
        let client = ctx.mixnet_signing_client()?;
        let migrate_fut = client.migrate(
            address,
            code_id,
            &migrate_msg,
            "contract migration from network-manager",
            None,
        );
        let migrate_res = async_with_progress(migrate_fut, &pb).await?;
        ctx.network.contracts.mixnet.migrate_info = Some(migrate_res.into());
        pb.println(format!("\t{name} contract has been migrated"));

        // migrate multisig
        pb.set_prefix("[2/2]");
        let name = &ctx.network.contracts.cw3_multisig.name;
        let code_id = ctx.network.contracts.cw3_multisig.upload_info()?.code_id;
        let address = ctx.network.contracts.cw3_multisig.address()?;
        pb.set_message(format!("attempting to migrate {name} contract..."));
        let migrate_msg = self.multisig_migrate_message(ctx)?;
        let client = ctx.multisig_signing_client()?;
        let migrate_fut = client.migrate(
            address,
            code_id,
            &migrate_msg,
            "contract migration from network-manager",
            None,
        );
        let migrate_res = async_with_progress(migrate_fut, &pb).await?;
        ctx.network.contracts.cw3_multisig.migrate_info = Some(migrate_res.into());
        pb.println(format!("\t{name} contract has been migrated"));

        pb.println("\t✅ performed all the needed migrations!");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    async fn get_build_info(&self, ctx: &mut Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();
        pb.println(format!(
            "🏗️ {}Obtaining contracts build information",
            style("[7/8]").bold().dim()
        ));

        let total = ctx.network.contracts.count() as u64;

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
            let build_info = async_with_progress(build_info_fut, &pb)
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

        pb.println("\t✅ updated all contract metadata!");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    async fn persist_in_database(&self, ctx: &Ctx) -> Result<(), NetworkManagerError> {
        let pb = ctx.spinner_pb();
        pb.println(format!(
            "📦 {}Storing all the results in the database",
            style("[8/8]").bold().dim()
        ));

        pb.set_message("attempting to persist network data...");
        let save_future = self.storage.persist_network(&ctx.network);
        async_with_progress(save_future, &pb).await?;

        pb.println("\t✅ the network information got persisted in the database for future use");
        pb.finish_and_clear();
        ctx.progress_bar.remove(&pb);
        Ok(())
    }

    pub(crate) async fn initialise_new_network<P: AsRef<Path>>(
        &self,
        contracts: P,
        network_name: Option<String>,
    ) -> Result<Network, NetworkManagerError> {
        let network_name = self.get_network_name(network_name);
        let mut ctx = Ctx::new(network_name, self.admin.deref().clone(), &self.rpc_endpoint)?;

        ctx.progress_bar.println(format!(
            "\n🚀 setting up new testnet '{}' over {}",
            ctx.network.name, self.rpc_endpoint
        ))?;

        let started = Instant::now();

        self.find_contracts(&mut ctx, contracts)?;
        self.upload_contracts(&mut ctx).await?;
        self.create_contract_admins_mnemonics(&mut ctx)?;
        self.transfer_admin_tokens(&ctx).await?;
        self.instantiate_contracts(&mut ctx).await?;
        self.perform_final_migrations(&mut ctx).await?;
        self.get_build_info(&mut ctx).await?;
        self.persist_in_database(&ctx).await?;

        ctx.progress_bar
            .println(format!("✨ Done in {}", HumanDuration(started.elapsed())))?;
        ctx.progress_bar.clear()?;

        Ok(ctx.into_network())
    }

    pub(crate) async fn load_existing_network(
        &self,
        network_name: String,
    ) -> Result<LoadedNetwork, NetworkManagerError> {
        self.storage.try_load_network(&network_name).await
    }
}
