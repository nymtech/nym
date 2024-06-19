// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{async_with_progress, ProgressTracker};
use crate::manager::dkg_skip::EcashSignerWithPaths;
use crate::manager::network::LoadedNetwork;
use crate::manager::NetworkManager;
use console::style;
use nym_compact_ecash::KeyPairAuth;
use nym_config::{
    must_get_home, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_NYM_APIS_DIR, NYM_DIR,
};
use nym_pemstore::traits::PemStorableKey;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
pub struct RunCommands(Vec<String>);

impl Display for RunCommands {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for cmd in &self.0 {
            writeln!(f, "{cmd}")?
        }
        Ok(())
    }
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

struct LocalApisCtx<'a> {
    nym_api_binary: PathBuf,
    progress: ProgressTracker,
    network: &'a LoadedNetwork,
    signers: Vec<EcashSignerWithPaths>,
}

impl<'a> LocalApisCtx<'a> {
    fn signer_id(&self, signer: &EcashSignerWithPaths) -> String {
        format!(
            "{}-{}",
            signer.data.cosmos_account.address, self.network.name
        )
    }

    fn println<I: AsRef<str>>(&self, msg: I) {
        self.progress.println(msg)
    }

    #[allow(unused)]
    fn set_pb_prefix(&self, prefix: impl Into<Cow<'static, str>>) {
        self.progress.set_pb_prefix(prefix)
    }

    fn set_pb_message(&self, msg: impl Into<Cow<'static, str>>) {
        self.progress.set_pb_message(msg)
    }

    async fn async_with_progress<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T>,
    {
        async_with_progress(fut, &self.progress.progress_bar).await
    }

    fn new(
        nym_api_binary: PathBuf,
        network: &'a LoadedNetwork,
        signers: Vec<EcashSignerWithPaths>,
    ) -> Result<Self, NetworkManagerError> {
        let progress = ProgressTracker::new(format!(
            "\n🚀 setting up new local signing nym-APIs for network '{}' over {}",
            network.name, network.rpc_endpoint
        ));

        Ok(LocalApisCtx {
            nym_api_binary,
            network,
            progress,
            signers,
        })
    }
}

impl NetworkManager {
    fn nym_api_config(&self, api_id: &str) -> PathBuf {
        must_get_home()
            .join(NYM_DIR)
            .join(DEFAULT_NYM_APIS_DIR)
            .join(api_id)
            .join(DEFAULT_CONFIG_DIR)
            .join(DEFAULT_CONFIG_FILENAME)
    }

    async fn initialise_api<'a>(
        &self,
        ctx: &LocalApisCtx<'a>,
        info: &EcashSignerWithPaths,
    ) -> Result<(), NetworkManagerError> {
        let address = &info.data.cosmos_account.address;

        ctx.set_pb_message(format!("initialising api {address}..."));

        let id = ctx.signer_id(info);

        // setup the binary itself
        let mut child = Command::new(&ctx.nym_api_binary)
            .args([
                "init",
                "--id",
                &id,
                "--nyxd-validator",
                ctx.network.rpc_endpoint.as_ref(),
                "--mnemonic",
                &Zeroizing::new(info.data.cosmos_account.mnemonic.to_string()),
                "--enable-zk-nym",
                "--announce-address",
                info.data.endpoint.as_ref(),
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let child_fut = child.wait();
        let out = ctx.async_with_progress(child_fut).await?;
        if !out.success() {
            return Err(NetworkManagerError::NymApiExecutionFailure);
        }

        // load the config (and do very nasty things to it)
        let config_path = self.nym_api_config(&id);
        let config_content = fs::read_to_string(config_path)?;
        let parsed_config: toml::Table = toml::from_str(&config_content)?;
        let storage_paths = &parsed_config["base"]
            .as_table()
            .expect("nym-api config serialisation has changed")["storage_paths"]
            .as_table()
            .expect("nym-api config serialisation has changed");

        let priv_id = &storage_paths["private_identity_key_file"]
            .as_str()
            .expect("nym-api config serialisation has changed");
        let pub_id = &storage_paths["public_identity_key_file"]
            .as_str()
            .expect("nym-api config serialisation has changed");
        let ecash = &parsed_config["coconut_signer"]
            .as_table()
            .expect("nym-api config serialisation has changed")["storage_paths"]
            .as_table()
            .expect("nym-api config serialisation has changed")["coconut_key_path"]
            .as_str()
            .expect("nym-api config serialisation has changed");

        // overwrite pre-generated files
        let fake_ecash_key = FakeDkgKey::new(&info.data.ecash_keypair);
        nym_pemstore::store_key(&fake_ecash_key, ecash)?;
        fs::copy(&info.paths.ed25519_keys.private_key_path, priv_id)?;
        fs::copy(&info.paths.ed25519_keys.public_key_path, pub_id)?;

        ctx.println(format!("\t nym-API {address} got initialised"));

        Ok(())
    }

    async fn initialise_apis<'a>(&self, ctx: &LocalApisCtx<'a>) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🔏 {}Initialising local nym-apis...",
            style("[1/1]").bold().dim()
        ));

        for signer in &ctx.signers {
            self.initialise_api(ctx, signer).await?
        }

        ctx.println("\t✅ all APIs got initialised!");
        Ok(())
    }

    fn prepare_run_commands<P: AsRef<Path>>(
        &self,
        ctx: &LocalApisCtx,
        env_file: P,
    ) -> Result<RunCommands, NetworkManagerError> {
        let bin_canon = fs::canonicalize(&ctx.nym_api_binary)?;
        let env_canon = fs::canonicalize(env_file)?;
        let bin_canon_display = bin_canon.display();
        let env_canon_display = env_canon.display();

        let mut cmds = Vec::new();
        for signer in &ctx.signers {
            let port = signer.api_port();
            let id = ctx.signer_id(signer);

            cmds.push(format!(
                "\tROCKET_PORT={port} {bin_canon_display} -c {env_canon_display} run --id {id}"
            ));
        }
        Ok(RunCommands(cmds))
    }

    fn output_run_commands(&self, ctx: &LocalApisCtx, cmds: &RunCommands) {
        ctx.println("🏇 run the apis with the following commands:");
        for cmd in &cmds.0 {
            ctx.println(cmd)
        }
    }

    fn prepare_env_file<P: AsRef<Path>>(
        &self,
        ctx: &LocalApisCtx,
        env_file: P,
    ) -> Result<(), NetworkManagerError> {
        let base_env = ctx.network.to_env_file_section();
        let updated_env = format!("{base_env}NYM_API={}", ctx.signers[0].data.endpoint);

        let env_file = env_file.as_ref();
        if let Some(parent) = env_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut env_file = File::create(env_file)?;
        env_file.write_all(updated_env.as_bytes())?;

        Ok(())
    }

    pub(crate) async fn setup_local_apis<P: AsRef<Path>>(
        &self,
        nym_api_binary: P,
        network: &LoadedNetwork,
        signer_data: Vec<EcashSignerWithPaths>,
    ) -> Result<RunCommands, NetworkManagerError> {
        let ctx = LocalApisCtx::new(nym_api_binary.as_ref().to_path_buf(), network, signer_data)?;
        let env_file = ctx.network.default_env_file_path();

        self.initialise_apis(&ctx).await?;
        self.prepare_env_file(&ctx, &env_file)?;
        let cmds = self.prepare_run_commands(&ctx, env_file)?;
        self.output_run_commands(&ctx, &cmds);

        Ok(cmds)
    }
}
