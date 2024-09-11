// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{ProgressCtx, ProgressTracker};
use crate::manager::contract::Account;
use crate::manager::network::LoadedNetwork;
use crate::manager::NetworkManager;
use console::style;
use dkg_bypass_contract::msg::FakeDealerData;
use nym_compact_ecash::{ttp_keygen, Base58, KeyPairAuth};
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::Addr;
use nym_pemstore::traits::PemStorableKey;
use nym_pemstore::{store_key, store_keypair, KeyPairPath};
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, GroupSigningClient, PagedGroupQueryClient,
};
use nym_validator_client::nyxd::cosmwasm::ContractCodeId;
use nym_validator_client::nyxd::cw4::Member;
use nym_validator_client::nyxd::{AccountId, CosmWasmClient};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use rand::rngs::OsRng;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use url::Url;
use zeroize::Zeroizing;

pub(crate) struct EcashSigner {
    pub(crate) ed25519_keypair: ed25519::KeyPair,
    pub(crate) ecash_keypair: nym_compact_ecash::KeyPairAuth,
    pub(crate) cosmos_account: Account,
    pub(crate) endpoint: Url,
}

#[derive(Default)]
pub(crate) struct EcashSignerPaths {
    pub(crate) ecash_key: PathBuf,
    pub(crate) ed25519_keys: KeyPairPath,
    pub(crate) mnemonic_path: PathBuf,
    pub(crate) endpoint_path: PathBuf,
}

pub(crate) struct EcashSignerWithPaths {
    pub(crate) data: EcashSigner,
    pub(crate) paths: EcashSignerPaths,
}

// perform the same serialisation as the nym-api keys
struct FakeDkgKey<'a> {
    inner: &'a KeyPairAuth,
}

impl<'a> FakeDkgKey<'a> {
    fn new(inner: &'a KeyPairAuth) -> Self {
        FakeDkgKey { inner }
    }
}

impl<'a> PemStorableKey for FakeDkgKey<'a> {
    type Error = NetworkManagerError;

    fn pem_type() -> &'static str {
        "ECASH KEY WITH EPOCH"
    }

    fn to_bytes(&self) -> Vec<u8> {
        // our fake key is ALWAYS issued for epoch 0
        let mut bytes = vec![0u8; 8];
        bytes.append(&mut self.inner.secret_key().to_bytes());
        bytes
    }

    fn from_bytes(_: &[u8]) -> Result<Self, Self::Error> {
        unimplemented!("this is not meant to be ever called")
    }
}

impl EcashSignerWithPaths {
    pub(crate) fn api_port(&self) -> u16 {
        self.data.endpoint.port().unwrap()
    }
}

struct DkgSkipCtx<'a> {
    progress: ProgressTracker,
    network: &'a LoadedNetwork,
    dkg_admin: DirectSigningHttpRpcNyxdClient,
    ecash_signers: Vec<EcashSignerWithPaths>,
}

impl<'a> ProgressCtx for DkgSkipCtx<'a> {
    fn progress_tracker(&self) -> &ProgressTracker {
        &self.progress
    }
}

impl<'a> DkgSkipCtx<'a> {
    fn dkg_contract(&self) -> &AccountId {
        &self.network.contracts.dkg.address
    }

    fn new(network: &'a LoadedNetwork) -> Result<Self, NetworkManagerError> {
        let progress = ProgressTracker::new(format!(
            "\n🥷 attempting to skip DKG on network '{}'",
            network.name
        ));

        Ok(DkgSkipCtx {
            progress,
            dkg_admin: network.dkg_signing_client()?,
            network,
            ecash_signers: vec![],
        })
    }

    fn group_signing_client(&self) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        self.network.cw4_group_signing_client()
    }

    fn admin_signing_client(
        &self,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            self.network.client_config()?,
            self.network.rpc_endpoint.as_str(),
            mnemonic,
        )?)
    }
}

impl NetworkManager {
    fn generate_ecash_signer_data(
        &self,
        ctx: &mut DkgSkipCtx,
        api_endpoints: Vec<Url>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "📝 {}Generating ecash keys for all signers...",
            style("[1/8]").bold().dim()
        ));

        // generate required materials
        let n = api_endpoints.len();
        let threshold = (2 * n + 3 - 1) / 3;

        let ecash_keys = ttp_keygen(threshold as u64, n as u64)?;

        let mut ecash_signers = Vec::new();
        let mut rng = OsRng;
        for (endpoint, ecash_keypair) in api_endpoints.into_iter().zip(ecash_keys.into_iter()) {
            let ed25519_keypair = ed25519::KeyPair::new(&mut rng);
            let data = EcashSigner {
                ed25519_keypair,
                ecash_keypair,
                cosmos_account: Account::new(),
                endpoint,
            };
            ctx.println(format!(
                "\t{} will be managed by {}",
                data.endpoint, data.cosmos_account.address
            ));
            let full = EcashSignerWithPaths {
                data,
                paths: EcashSignerPaths::default(),
            };
            ecash_signers.push(full)
        }
        ctx.ecash_signers = ecash_signers;

        ctx.println("\t✅ generated ecash keys for all signers");
        Ok(())
    }

    async fn validate_existing_contracts<'a>(
        &self,
        ctx: &DkgSkipCtx<'a>,
    ) -> Result<ContractCodeId, NetworkManagerError> {
        ctx.println(format!(
            "🔬 {}Validating the current DKG and group contracts...",
            style("[2/8]").bold().dim()
        ));

        ctx.set_pb_prefix("[1/3]");
        ctx.set_pb_message("checking DKG epoch data...");
        let epoch_fut = ctx.dkg_admin.get_current_epoch();
        let dkg_epoch = ctx.async_with_progress(epoch_fut).await?;
        if dkg_epoch.epoch_id != 0 {
            return Err(NetworkManagerError::NonZeroEpoch);
        }

        if !dkg_epoch.state.is_waiting_initialisation() {
            return Err(NetworkManagerError::DkgAlreadyStarted);
        }

        ctx.set_pb_prefix("[2/3]");
        ctx.set_pb_message("retrieving DKG contract code_id...");
        let code_fut = ctx
            .dkg_admin
            .get_contract_code_history(&ctx.network.contracts.dkg.address);
        let code_history = ctx.async_with_progress(code_fut).await?;

        // SAFETY:
        // if this is empty our abci query is invalid since we have just queried the contract so it must exist
        let current_code = code_history.last().unwrap().code_id;
        ctx.println("\tthe DKG contract is all good!");

        ctx.set_pb_prefix("[3/3]");
        ctx.set_pb_message("checking cw4 group members data...");
        let members_fut = ctx.dkg_admin.get_all_members();
        let members = ctx.async_with_progress(members_fut).await?;
        if !members.is_empty() {
            return Err(NetworkManagerError::ExistingCW4Members);
        }

        ctx.println("\tthe group contract is all good!");
        ctx.println("\t✅ the existing contracts are all good!");

        Ok(current_code)
    }

    async fn persist_dkg_keys<'a, P: AsRef<Path>>(
        &self,
        ctx: &mut DkgSkipCtx<'a>,
        output_dir: P,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "📦 {}Persisting the signer keys...",
            style("[3/8]").bold().dim()
        ));

        ctx.set_pb_message("storing the signer data on disk...");

        let output_dir = output_dir.as_ref();
        let pb = &ctx.progress.progress_bar;

        for signer in &mut ctx.ecash_signers {
            let address = &signer.data.cosmos_account.address;
            let url = &signer.data.endpoint;
            let signer_dir = output_dir.join(address.to_string());
            fs::create_dir_all(&signer_dir)?;

            let fake_ecash_key = FakeDkgKey::new(&signer.data.ecash_keypair);

            let ecash_path = signer_dir.join("ecash");

            let ed25519_paths = KeyPairPath {
                private_key_path: signer_dir.join("ed25519"),
                public_key_path: signer_dir.join("ed25519.pub"),
            };

            let mnemonic_path = signer_dir.join("mnemonic");
            let endpoint_path = signer_dir.join("announce_address");

            store_key(&fake_ecash_key, &ecash_path)?;
            store_keypair(&signer.data.ed25519_keypair, &ed25519_paths)?;

            fs::write(
                &mnemonic_path,
                Zeroizing::new(signer.data.cosmos_account.mnemonic.to_string()),
            )?;
            fs::write(&endpoint_path, url.as_str())?;

            signer.paths.ecash_key = ecash_path;
            signer.paths.ed25519_keys = ed25519_paths;
            signer.paths.mnemonic_path = mnemonic_path;
            signer.paths.endpoint_path = endpoint_path;

            pb.println(format!(
                "\tpersisted {address} (endpoint: {url}) data under {}",
                signer_dir.display()
            ));
        }

        ctx.println("\t✅ persisted all the signer keys!");
        Ok(())
    }

    async fn upload_bypass_contract<'a, P: AsRef<Path>>(
        &self,
        ctx: &DkgSkipCtx<'a>,
        dkg_bypass_contract: P,
    ) -> Result<ContractCodeId, NetworkManagerError> {
        ctx.println(format!(
            "🚚 {}Uploading the bypass contract...",
            style("[4/8]").bold().dim()
        ));

        ctx.set_pb_message("uploading the bypass contract...");

        let res = self
            .upload_contract(
                &ctx.dkg_admin,
                &ctx.progress.progress_bar,
                dkg_bypass_contract,
            )
            .await?;

        ctx.println("\t✅ uploaded the bypass contract!");

        Ok(res.code_id)
    }

    async fn migrate_to_bypass_contract<'a>(
        &self,
        ctx: &DkgSkipCtx<'a>,
        code_id: ContractCodeId,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🔀 {}Attempting to migrate into the bypass contract...",
            style("[5/8]").bold().dim()
        ));

        ctx.set_pb_message("migrating the DKG contract...");

        let migrate_msg = dkg_bypass_contract::MigrateMsg {
            dealers: ctx
                .ecash_signers
                .iter()
                .map(|signer| FakeDealerData {
                    vk: signer.data.ecash_keypair.verification_key().to_bs58(),
                    ed25519_identity: signer.data.ed25519_keypair.public_key().to_base58_string(),
                    announce: signer.data.endpoint.to_string(),
                    owner: Addr::unchecked(signer.data.cosmos_account.address.as_ref()),
                })
                .collect(),
        };

        let migrate_fut = ctx.dkg_admin.migrate(
            ctx.dkg_contract(),
            code_id,
            &migrate_msg,
            "migrating bypass DKG contract from testnet-manager",
            None,
        );
        ctx.async_with_progress(migrate_fut).await?;

        ctx.println("\t✅ migrated the DKG into the bypass contract!");

        Ok(())
    }

    async fn restore_dkg_contract<'a>(
        &self,
        ctx: &DkgSkipCtx<'a>,
        code_id: ContractCodeId,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "↩️ {}Attempting to migrate back into the original DKG contract...",
            style("[6/8]").bold().dim()
        ));

        ctx.set_pb_message("migrating the DKG contract...");

        let migrate_msg = nym_coconut_dkg_common::msg::MigrateMsg {};
        let migrate_fut = ctx.dkg_admin.migrate(
            ctx.dkg_contract(),
            code_id,
            &migrate_msg,
            "migrating initial DKG contract from testnet-manager",
            None,
        );
        ctx.async_with_progress(migrate_fut).await?;

        ctx.println("\t✅ restored the original DKG contract!");

        Ok(())
    }

    async fn add_group_members<'a>(&self, ctx: &DkgSkipCtx<'a>) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "👪 {}Adding all the cw4 group members...",
            style("[7/8]").bold().dim()
        ));

        ctx.set_pb_message("⛽creating a new big cw4 family...");
        let admin = ctx.group_signing_client()?;
        let new_members = ctx
            .ecash_signers
            .iter()
            .map(|s| Member {
                addr: s.data.cosmos_account.address.to_string(),
                weight: 1,
            })
            .collect();

        let update_fut = admin.update_members(new_members, Vec::new(), None);

        ctx.async_with_progress(update_fut).await?;
        ctx.println("\t✅ new cw4 group members got added");
        Ok(())
    }

    async fn transfer_signer_tokens<'a>(
        &self,
        ctx: &DkgSkipCtx<'a>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "💸 {}Transferring tokens to the new signers...",
            style("[8/8]").bold().dim()
        ));

        let admin = ctx.admin_signing_client(self.admin.deref().clone())?;

        let mut receivers = Vec::new();
        for signer in &ctx.ecash_signers {
            // send 101nym to the admin
            receivers.push((
                signer.data.cosmos_account.address.clone(),
                admin.mix_coins(101_000000),
            ))
        }

        ctx.set_pb_message("attempting to send signer tokens...");

        let send_future = admin.send_multiple(
            receivers,
            "signers token transfer from testnet-manager",
            None,
        );
        let res = ctx.async_with_progress(send_future).await?;

        ctx.println(format!(
            "\t✅ sent tokens in transaction: {} (height {})",
            res.hash, res.height
        ));
        Ok(())
    }

    pub(crate) async fn attempt_bypass_dkg<P1, P2>(
        &self,
        api_endpoints: Vec<Url>,
        network: &LoadedNetwork,
        dkg_bypass_contract: P1,
        data_output_dir: P2,
    ) -> Result<Vec<EcashSignerWithPaths>, NetworkManagerError>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        if api_endpoints.is_empty() {
            return Err(NetworkManagerError::NoApiEndpoints);
        }

        let dkg_bypass_contract = dkg_bypass_contract.as_ref();
        if !dkg_bypass_contract.is_file() {
            return Err(NetworkManagerError::MalformedDkgBypassContractPath);
        }
        let Some(ext) = dkg_bypass_contract.extension() else {
            return Err(NetworkManagerError::MalformedDkgBypassContractPath);
        };
        if ext != "wasm" {
            return Err(NetworkManagerError::MalformedDkgBypassContractPath);
        }

        let mut ctx = DkgSkipCtx::new(network)?;

        self.generate_ecash_signer_data(&mut ctx, api_endpoints)?;
        let current_code_id = self.validate_existing_contracts(&ctx).await?;
        self.persist_dkg_keys(&mut ctx, data_output_dir).await?;
        let new_code_id = self
            .upload_bypass_contract(&ctx, dkg_bypass_contract)
            .await?;
        self.migrate_to_bypass_contract(&ctx, new_code_id).await?;
        self.restore_dkg_contract(&ctx, current_code_id).await?;
        self.add_group_members(&ctx).await?;
        self.transfer_signer_tokens(&ctx).await?;

        Ok(ctx.ecash_signers)
    }
}
