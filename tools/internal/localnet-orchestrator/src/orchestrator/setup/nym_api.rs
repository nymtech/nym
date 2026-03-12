// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    CARGO_REGISTRY_CACHE_VOLUME, CONTAINER_NETWORK_NAME, CONTRACTS_CACHE_VOLUME,
    NYM_API_UTILITY_BEARER, contract_build_names,
};
use crate::helpers::monorepo_root_path;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::container_helpers::{
    check_container_image_exists, default_nym_binaries_image_tag, get_container_ip_address,
    load_image_into_container_runtime, run_container, save_docker_image,
};
use crate::orchestrator::context::LocalnetContext;
use crate::orchestrator::cosmwasm_contract::ContractBeingInitialised;
use crate::orchestrator::state::LocalnetState;
use anyhow::{Context, bail};
use dkg_bypass_contract::msg::FakeDealerData;
use nym_coconut_dkg_common::types::Addr;
use nym_compact_ecash::{Base58, KeyPairAuth, ttp_keygen};
use nym_crypto::asymmetric::ed25519;
use nym_pemstore::traits::PemStorableKey;
use nym_pemstore::{KeyPairPath, store_key, store_keypair};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, GroupSigningClient, PagedGroupQueryClient,
};
use nym_validator_client::nyxd::cw4::Member;
use rand::{CryptoRng, Rng, thread_rng};
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tracing::{debug, info};

// perform the same serialisation as the nym-api keys
struct FakeDkgKey<'a> {
    inner: &'a KeyPairAuth,
}

impl<'a> FakeDkgKey<'a> {
    fn new(inner: &'a KeyPairAuth) -> Self {
        FakeDkgKey { inner }
    }
}

impl PemStorableKey for FakeDkgKey<'_> {
    type Error = std::io::Error;

    fn pem_type() -> &'static str {
        "ECASH KEY WITH EPOCH"
    }

    fn to_bytes(&self) -> Vec<u8> {
        // our fake key is ALWAYS issued for epoch 0
        let mut bytes = vec![0u8; 8];
        bytes.append(&mut self.inner.secret_key().to_bytes());
        bytes
    }

    #[allow(clippy::unimplemented)]
    fn from_bytes(_: &[u8]) -> Result<Self, Self::Error> {
        unimplemented!("this is not meant to be ever called")
    }
}

pub(crate) struct Config {
    pub(crate) cosmwasm_optimizer_image: String,
    pub(crate) monorepo_root: Option<PathBuf>,
    pub(crate) custom_dns: Option<String>,
    pub(crate) allow_cached_build: bool,
}

struct DKGKeys {
    ecash_keys: KeyPairAuth,
    ed25519_keypair: ed25519::KeyPair,
}

impl DKGKeys {
    pub(crate) fn generate<R: Rng + CryptoRng>(rng: &mut R) -> anyhow::Result<Self> {
        let ecash_keys = ttp_keygen(1, 1)
            .context("ecash key generation failure")?
            .pop()
            .context("empty ecash keys")?;

        let ed25519_keypair = ed25519::KeyPair::new(rng);
        Ok(DKGKeys {
            ed25519_keypair,
            ecash_keys,
        })
    }
}

struct NymApiSetup {
    allow_cached_build: bool,
    cosmwasm_optimizer_image: String,
    monorepo_root: PathBuf,
    nym_binaries_image_location: NamedTempFile,
    dkg_key_location: NamedTempFile,
    ed25519_private_key_location: NamedTempFile,
    ed25519_public_key_location: NamedTempFile,
    dkg_bypass_contract: ContractBeingInitialised,
    dkg_keys: Option<DKGKeys>,
    custom_dns: Option<String>,
}

impl NymApiSetup {
    pub(crate) fn new(config: Config) -> anyhow::Result<Self> {
        let monorepo_root = monorepo_root_path(config.monorepo_root)?;

        Ok(NymApiSetup {
            custom_dns: config.custom_dns,
            allow_cached_build: config.allow_cached_build,
            cosmwasm_optimizer_image: config.cosmwasm_optimizer_image,
            monorepo_root,
            nym_binaries_image_location: NamedTempFile::new()?,
            dkg_key_location: NamedTempFile::new()?,
            ed25519_private_key_location: NamedTempFile::new()?,
            ed25519_public_key_location: NamedTempFile::new()?,
            dkg_bypass_contract: ContractBeingInitialised::new("dkg-bypass-contract"),
            dkg_keys: None,
        })
    }

    pub(crate) fn image_temp_location_arg(&self) -> anyhow::Result<&str> {
        self.nym_binaries_image_location
            .path()
            .to_str()
            .context("invalid temporary file location")
    }

    pub(crate) fn nym_binaries_dockerfile_location_canon(&self) -> anyhow::Result<PathBuf> {
        Ok(self
            .monorepo_root
            .join("docker")
            .join("localnet")
            .join("nym-binaries-localnet.Dockerfile")
            .canonicalize()?)
    }

    pub(crate) fn monorepo_root_canon(&self) -> anyhow::Result<PathBuf> {
        Ok(self.monorepo_root.canonicalize()?)
    }

    pub(crate) fn dkg_keys(&self) -> anyhow::Result<&DKGKeys> {
        self.dkg_keys.as_ref().context("missing dkg keys")
    }
}

impl LocalnetOrchestrator {
    pub(crate) fn expected_bypass_contract_wasm_path(&self) -> PathBuf {
        self.storage
            .cosmwasm_contracts_directory()
            .join(contract_build_names::DKG_BYPASS_CONTRACT)
    }

    async fn build_nym_binaries_docker_image(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        let dockerfile_path = ctx.data.nym_binaries_dockerfile_location_canon()?;
        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;

        self.try_build_nym_binaries_docker_image(ctx, dockerfile_path, monorepo_path, &image_tag)
            .await
    }

    async fn save_nym_binaries_docker_image(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        let output_path = ctx.data.image_temp_location_arg()?.to_owned();
        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;

        save_docker_image(ctx, &output_path, &image_tag).await
    }

    async fn load_nym_binaries_into_container_runtime(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        let image_path = ctx.data.image_temp_location_arg()?.to_owned();
        load_image_into_container_runtime(ctx, &image_path).await
    }

    async fn verify_nym_binaries_image(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("verifying localnet-nym-binaries container image...", "❔");
        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;

        if !check_container_image_exists(ctx, &image_tag).await? {
            bail!("localnet-nym-binaries image verification failed");
        }
        Ok(())
    }

    fn generate_dkg_keys(&self, ctx: &mut LocalnetContext<NymApiSetup>) -> anyhow::Result<()> {
        let dkg_keys = DKGKeys::generate(&mut thread_rng())?;
        let fake_ecash_key = FakeDkgKey::new(&dkg_keys.ecash_keys);

        let ed25519_paths = KeyPairPath {
            private_key_path: ctx.data.ed25519_private_key_location.path().to_owned(),
            public_key_path: ctx.data.ed25519_public_key_location.path().to_owned(),
        };

        store_key(&fake_ecash_key, &ctx.data.dkg_key_location)?;
        store_keypair(&dkg_keys.ed25519_keypair, &ed25519_paths)?;
        ctx.data.dkg_keys = Some(dkg_keys);

        Ok(())
    }

    fn dkg_admin_signer(&self) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let mnemonic = &self.localnet_details.contracts()?.dkg.admin.mnemonic;
        self.signing_client(mnemonic)
    }

    fn group_admin_signer(&self) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let mnemonic = &self.localnet_details.contracts()?.cw4_group.admin.mnemonic;
        self.signing_client(mnemonic)
    }

    async fn validate_dkg_contracts_state(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("verifying DKG and group contract states...", "🤔");

        let client = self.rpc_query_client()?;

        ctx.set_pb_prefix("[1/2]");
        ctx.set_pb_message("checking DKG epoch data...");
        let epoch_fut = client.get_current_epoch();
        let dkg_epoch = ctx.async_with_progress(epoch_fut).await?;
        if dkg_epoch.epoch_id != 0 {
            bail!("DKG epoch has already progressed")
        }

        if !dkg_epoch.state.is_waiting_initialisation() {
            bail!("DKG has already started");
        }

        ctx.set_pb_prefix("[2/2]");
        ctx.set_pb_message("checking cw4 group members data...");
        let members_fut = client.get_all_members();
        let members = ctx.async_with_progress(members_fut).await?;
        if !members.is_empty() {
            bail!("CW4 multisig group is not empty!")
        }

        Ok(())
    }

    fn check_bypass_contract_built(&self, ctx: &LocalnetContext<NymApiSetup>) -> bool {
        // check cache if possible
        if ctx.data.allow_cached_build
            && self
                .storage
                .data_cache()
                .cached_contract_exists(contract_build_names::DKG_BYPASS_CONTRACT)
        {
            return true;
        }

        // fallback to default
        self.expected_bypass_contract_wasm_path().exists()
    }

    async fn build_dkg_bypass_contract(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("building the DKG bypass contract...", "🏗️");

        ctx.execute_cmd_with_exit_status("docker", [
            "run",
            "--rm",
            "-v",
            &format!("{}:/code", ctx.data.monorepo_root.to_string_lossy()),
            "--mount",
            &format!("type=volume,source={CONTRACTS_CACHE_VOLUME},target=/target"),
            "--mount",
            &format!(
                "type=volume,source={CARGO_REGISTRY_CACHE_VOLUME},target=/usr/local/cargo/registry"
            ),
            &ctx.data.cosmwasm_optimizer_image,
            "tools/internal/localnet-orchestrator/dkg-bypass-contract" // relative path to the contract code from the monorepo root
        ]).await?;

        let source = ctx
            .data
            .monorepo_root
            .join("artifacts")
            .join("dkg_bypass_contract.wasm");
        let target = self.expected_bypass_contract_wasm_path();
        debug!("moving {} to {}", source.display(), target.display());

        if !source.exists() {
            bail!("source ({}) does not exist", source.display());
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        std::fs::rename(&source, &target)?;

        // copy it to cache as well
        let cache_path = self
            .storage
            .data_cache()
            .cached_contract_path(contract_build_names::DKG_BYPASS_CONTRACT);
        fs::copy(&target, &cache_path)?;

        ctx.data.dkg_bypass_contract.wasm_path = Some(target);
        Ok(())
    }

    async fn upload_dkg_bypass_contract(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("uploading the DKG bypass contract...", "🚚");

        let cache = self.storage.data_cache();

        let path = if ctx.data.allow_cached_build
            && cache.cached_contract_exists(contract_build_names::DKG_BYPASS_CONTRACT)
        {
            cache.cached_contract_path(contract_build_names::DKG_BYPASS_CONTRACT)
        } else {
            self.expected_bypass_contract_wasm_path()
        };

        let upload_res = self.upload_contract(ctx, path).await?;
        ctx.println(format!(
            "\tdkg bypass contract uploaded with code: {}. tx: {}",
            upload_res.code_id, upload_res.transaction_hash
        ));
        ctx.data.dkg_bypass_contract.upload_info = Some(upload_res.into());
        Ok(())
    }

    async fn migrate_to_bypass_contract(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("migrating into the DKG bypass contract...", "🔀");

        let keys = ctx.data.dkg_keys()?;
        let api_endpoint = self.localnet_details.nym_api_endpoint()?;
        let api_address = self
            .localnet_details
            .auxiliary_accounts()?
            .mixnet_rewarder
            .address();

        let migrate_msg = dkg_bypass_contract::MigrateMsg {
            dealers: vec![FakeDealerData {
                vk: keys.ecash_keys.verification_key().to_bs58(),
                ed25519_identity: keys.ed25519_keypair.public_key().to_base58_string(),
                announce: api_endpoint.to_string(),
                owner: Addr::unchecked(api_address.as_ref()),
            }],
        };

        let dkg_contract = &self.localnet_details.contracts()?.dkg;

        let dkg_admin = self.dkg_admin_signer()?;
        let migrate_fut = dkg_admin.migrate(
            &dkg_contract.address,
            ctx.data.dkg_bypass_contract.upload_info()?.code_id,
            &migrate_msg,
            "migrating bypass DKG contract from localnet orchestrator",
            None,
        );
        ctx.async_with_progress(migrate_fut).await?;

        Ok(())
    }

    async fn restore_dkg_contract(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("restoring the original DKG contract...", "↩️");

        // retrieve DKG's original code (which will be the penultimate one)
        let client = self.rpc_query_client()?;

        let code_fut =
            client.get_contract_code_history(&self.localnet_details.contracts()?.dkg.address);
        let code_history = ctx.async_with_progress(code_fut).await?;
        let entries = code_history.len();
        let code_id = code_history
            .get(entries - 2)
            .context("dkg contract has not been initialised")?
            .code_id;

        let dkg_admin = self.dkg_admin_signer()?;

        let migrate_msg = nym_coconut_dkg_common::msg::MigrateMsg {};
        let migrate_fut = dkg_admin.migrate(
            &self.localnet_details.contracts()?.dkg.address,
            code_id,
            &migrate_msg,
            "restoring original DKG contract",
            None,
        );
        ctx.async_with_progress(migrate_fut).await?;

        Ok(())
    }

    async fn add_dkg_group_members(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("adding all the cw4 group members...", "👪️");

        ctx.set_pb_message("⛽ creating a new big cw4 family...");
        let admin = self.group_admin_signer()?;
        let signer = &self.localnet_details.auxiliary_accounts()?.mixnet_rewarder;
        let new_members = vec![Member {
            addr: signer.address.to_string(),
            weight: 1,
        }];

        let update_fut = admin.update_members(new_members, Vec::new(), None);

        ctx.async_with_progress(update_fut).await?;
        ctx.println("\t✅ new cw4 group members got added");
        Ok(())
    }

    async fn initialise_nym_api_data(
        &self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("setting up nym api instance data...", "🖋️");

        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;

        // 1.1 retrieve nyxd ip
        ctx.set_pb_prefix("[1/3]");
        ctx.set_pb_message("retrieving nyxd container ip address...");

        let nyxd_container_ip = get_container_ip_address(ctx, &self.nyxd_container_name()).await?;
        let nyxd_endpoint = format!("http://{nyxd_container_ip}:26657");
        let mnemonic = self
            .localnet_details
            .auxiliary_accounts()?
            .mixnet_rewarder
            .mnemonic
            .to_string();

        // 1.2 generate incomplete .env file (but complete enough-ish for the API to start)
        let content = self.localnet_details.env_file_content()?;
        let env_path = self
            .storage
            .nym_api_container_data_directory()
            .join("localnet.env");
        fs::write(env_path, &content)?;

        // 3. run init
        ctx.set_pb_prefix("[2/3]");
        ctx.set_pb_message("initialising nym-api data...");

        run_container(
            ctx,
            [
                "--name",
                &self.nym_api_container_name(),
                "-v",
                &self.nym_api_volume(),
                "--network",
                CONTAINER_NETWORK_NAME,
                "--rm",
                &image_tag,
                "nym-api",
                "-c",
                "/root/.nym/nym-api/default/localnet.env",
                "init",
                "--nyxd-validator",
                &nyxd_endpoint,
                "--mnemonic",
                &mnemonic,
                "--enable-monitor",
                "--enable-rewarding",
                "--enable-zk-nym",
                "--allow-illegal-ips",
                "--utility-routes-bearer",
                NYM_API_UTILITY_BEARER,
                "--announce-address",
                "http://placeholder.nym",
            ],
            ctx.data.custom_dns.clone(),
        )
        .await?;

        // 3. copy keys
        ctx.set_pb_prefix("[3/3]");
        ctx.set_pb_message("injecting pre-generated DKG keys...");

        fs::copy(&ctx.data.dkg_key_location, self.storage.nym_api_ecash_key())?;
        fs::copy(
            &ctx.data.ed25519_private_key_location,
            self.storage.nym_api_ed25519_private_key(),
        )?;
        fs::copy(
            &ctx.data.ed25519_public_key_location,
            self.storage.nym_api_ed25519_public_key(),
        )?;

        Ok(())
    }

    // quite annoying https://github.com/apple/container/issues/282 is still not resolved
    async fn start_nym_api(
        &mut self,
        ctx: &mut LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("starting up nym-api...", "🚀");

        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;

        // I hate the fact we have to wait for a 'magic file' before the startup
        // but that's the best I could think of without redesigning the whole ecash controller
        // inside the nym api
        let startup_cmd = r#"
CONTAINER_IP=$(hostname -i);
while [ ! -f /root/.nym/nym-api/default/dkg_ready ]; do
    sleep 0.5;
done;

nym-api -c /root/.nym/nym-api/default/localnet.env run --allow-illegal-ips --announce-address http://${CONTAINER_IP}:8000"#;

        // 2. start the api in the background
        run_container(
            ctx,
            [
                "--name",
                &self.nym_api_container_name(),
                "-v",
                &self.nym_api_volume(),
                "--network",
                CONTAINER_NETWORK_NAME,
                "-d",
                &image_tag,
                "sh",
                "-c",
                startup_cmd,
            ],
            ctx.data.custom_dns.clone(),
        )
        .await?;

        // 3. retrieve its container ip address
        let nym_api_container_ip =
            get_container_ip_address(ctx, &self.nym_api_container_name()).await?;
        self.localnet_details
            .set_nym_api_endpoint(format!("http://{nym_api_container_ip}:8000").parse()?);

        Ok(())
    }

    fn mark_dkg_as_ready(&self, ctx: &mut LocalnetContext<NymApiSetup>) -> anyhow::Result<()> {
        ctx.begin_next_step(
            "creating magic file to inform nym-api of DKG being completed",
            "🪄",
        );

        let magic_file_location = self
            .storage
            .nym_api_container_data_directory()
            .join("dkg_ready");

        fs::write(magic_file_location, "")?;
        Ok(())
    }

    async fn finalize_nym_api_setup(
        &mut self,
        mut ctx: LocalnetContext<NymApiSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("persisting nym api details", "📝");

        // unfortunately we had to set `self.localnet_details.set_nym_api_endpoint` earlier due to
        // non-predictable container ip addresses, so we can't be consistent with other setup steps
        let address = self.localnet_details.nym_api_endpoint()?;
        self.storage
            .orchestrator()
            .save_nym_api_details(&self.localnet_details.human_name, address.as_str())
            .await?;
        self.state = LocalnetState::RunningNymApi;

        Ok(())
    }

    pub(crate) async fn initialise_nym_api(&mut self, config: Config) -> anyhow::Result<()> {
        let setup = NymApiSetup::new(config)?;
        let mut ctx = LocalnetContext::new(setup, 15, "\ninitialising nym-api with DKG keys");
        fs::create_dir_all(self.storage.nym_api_container_data_directory())
            .context("failed to create nym-api data directory")?;

        // 0.1 check if we have to do anything
        if self.check_nym_api_container_is_running(&ctx).await? {
            info!("nym-api instance for this localnet is already running");
            return Ok(());
        }

        // 0.2 check if container had already been built
        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_tag = default_nym_binaries_image_tag(&monorepo_path)?;
        if check_container_image_exists(&ctx, &image_tag).await? {
            info!(
                "'{image_tag}' container image already exists - skipping docker build and import",
            );
            ctx.skip_steps(4);
        } else {
            // 1. docker build
            self.build_nym_binaries_docker_image(&mut ctx).await?;

            // 2. docker save
            self.save_nym_binaries_docker_image(&mut ctx).await?;

            // 3. container load
            self.load_nym_binaries_into_container_runtime(&mut ctx)
                .await?;

            // 4. container image inspect
            self.verify_nym_binaries_image(&mut ctx).await?;
        }

        // 5. generate (and persist) all keys needed for dkg
        self.generate_dkg_keys(&mut ctx)?;

        // 6. initialise nym-api configs
        self.initialise_nym_api_data(&mut ctx).await?;

        // 7. ensure the current contracts are in the valid state, i.e. DKG hasn't been run,
        // the multisig group is empty, etc.
        self.validate_dkg_contracts_state(&mut ctx).await?;

        // 8. start nyxd in the background and retrieve the container ip address
        // (which is needed for the dkg bypass)
        self.start_nym_api(&mut ctx).await?;

        // 9.1 check if the contract has already been build
        if self.check_bypass_contract_built(&ctx) {
            info!("the dkg bypass contract has already been built - skipping the step");
            ctx.skip_steps(1)
        } else {
            // 9.2 build it
            self.build_dkg_bypass_contract(&mut ctx).await?;
        }

        // 10. upload the dkg state bypass contract
        // (to overwrite the current DKG state without having to actually perform the exchange)
        self.upload_dkg_bypass_contract(&mut ctx).await?;

        // 11. migrate current dkg contract state into the bypass contract
        // (keys are set in migrate msg)
        self.migrate_to_bypass_contract(&mut ctx).await?;

        // 12. restore the original DKG contract code
        self.restore_dkg_contract(&mut ctx).await?;

        // 13. add nym-api to the CW4 DKG group
        self.add_dkg_group_members(&mut ctx).await?;

        // 14. create tha magic file for nym-api to trigger its full startup
        self.mark_dkg_as_ready(&mut ctx)?;

        // 15. persist relevant information and update local state
        self.finalize_nym_api_setup(ctx).await?;

        Ok(())
    }
}
