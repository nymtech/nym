// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use clap::ArgGroup;
use clap::{Args, Parser, Subcommand};
use importer_contract::contract::EmptyMessage;
use importer_contract::{base85, ExecuteMsg};
use nym_bin_common::bin_info;
use nym_bin_common::logging::setup_tracing_logger;
use nym_network_defaults::{setup_env, NymNetworkDetails};
use nym_validator_client::nyxd::cosmwasm_client::types::{InstantiateOptions, Model};
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use serde::{Deserialize, Serialize};
use std::env::current_dir;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{debug, info};

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub struct Cli {
    /// Path pointing to an env file that configures the CLI.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    #[clap(long)]
    pub(crate) mnemonic: bip39::Mnemonic,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Serialize, Deserialize)]
pub struct CachedState {
    pub importer_address: AccountId,
    pub state_imported: bool,
}

impl CachedState {
    pub fn save(&self) -> anyhow::Result<()> {
        let path = cached_state_file();
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let file = File::create(&path)?;
        serde_json::to_writer_pretty(file, self)?;

        info!("saved cached details to {}", path.display());
        Ok(())
    }

    pub fn load() -> anyhow::Result<Self> {
        let file = File::open(cached_state_file())?;
        Ok(serde_json::from_reader(&file)?)
    }
}

fn cached_state_file() -> PathBuf {
    dirs::cache_dir()
        .unwrap()
        .join("contract-state-importer")
        .join(".state.json")
}

// this only works if the cli is called from somewhere within the nym directory
// (which realistically is going to be the case most of the time)
fn importer_contract_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(explicit) = explicit {
        return Ok(explicit);
    }

    for ancestor in current_dir()?.ancestors() {
        debug!("checking {:?}", fs::canonicalize(ancestor));
        for content in ancestor.read_dir()? {
            let dir_entry = content?;
            let Ok(name) = dir_entry.file_name().into_string() else {
                continue;
            };

            if name == "target" {
                let maybe_contract_path = dir_entry
                    .path()
                    .join("wasm32-unknown-unknown")
                    .join("release")
                    .join("importer_contract.wasm");

                if maybe_contract_path.exists() {
                    return Ok(maybe_contract_path);
                }
            }
        }
    }

    bail!("could not find importer_contract.wasm")
}

fn importer_contract_address(explicit: Option<AccountId>) -> anyhow::Result<AccountId> {
    if let Some(explicit) = explicit {
        return Ok(explicit);
    }

    let state = CachedState::load()?;
    Ok(state.importer_address)
}

#[derive(Args, Clone)]
pub struct PrepareArgs {
    /// Path to the .wasm file with the importer contract
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/importer_contract.wasm"
    #[clap(long)]
    pub importer_contract_path: Option<PathBuf>,
}

#[derive(Args, Clone)]
pub struct SetStateArgs {
    /// Explicit address of the initialised importer contract.
    /// If not set, the value from the cached state will be attempted to be used
    #[clap(long)]
    pub importer_contract_address: Option<AccountId>,

    /// Path to the file containing state dump of a cosmwasm contract
    #[clap(long)]
    pub raw_state: PathBuf,
}

#[derive(Args, Clone)]
#[clap(group(ArgGroup::new("contract").required(true)))]
pub struct SwapContractArgs {
    /// Explicit address of the initialised importer contract.
    /// If not set, the value from the cached state will be attempted to be used
    #[clap(long)]
    pub importer_contract_address: Option<AccountId>,

    /// Code id of the previously uploaded cosmwasm smart contract that will be applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_code_id: Option<u64>,

    /// Path to a cosmwasm smart contract that will be uploaded and applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_path: Option<PathBuf>,

    /// The custom migrate message used for migrating into target contract.
    /// If none is provided an empty object will be used instead, i.e. '{}'
    #[clap(long)]
    pub migrate_msg: Option<serde_json::Value>,
}

#[derive(Args, Clone)]
#[clap(group(ArgGroup::new("contract").required(true)))]
pub struct InitialiseWithStateArgs {
    /// Path to the .wasm file with the importer contract
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/importer_contract.wasm"
    #[clap(long)]
    pub importer_contract_path: Option<PathBuf>,

    /// Path to the file containing state dump of a cosmwasm contract
    #[clap(long)]
    pub raw_state: PathBuf,

    /// Code id of the previously uploaded cosmwasm smart contract that will be applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_code_id: Option<u64>,

    /// Path to a cosmwasm smart contract that will be uploaded and applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_path: Option<PathBuf>,

    #[clap(long)]
    pub migrate_msg: Option<serde_json::Value>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Upload and instantiates the importer contract
    PrepareContract(PrepareArgs),

    /// Set the state of the previously instantiated importer contract with the provided state dump
    SetState(SetStateArgs),

    /// Swap the importer contract code with the one corresponding to the previously uploaded state dump
    SwapContract(SwapContractArgs),

    /// Combines the functionalities of `prepare-contract`, `set-state` and `swap-contract`
    InitialiseWithState(InitialiseWithStateArgs),
}

async fn create_importer_contract(
    explicit_contract_path: Option<PathBuf>,
    client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<AccountId> {
    info!("attempting to create the importer contract");

    let importer_path = importer_contract_path(explicit_contract_path)?;
    info!(
        "going to use the following importer contract: '{}'",
        fs::canonicalize(&importer_path)?.display()
    );

    let mut data = Vec::new();
    File::open(importer_path)?.read_to_end(&mut data)?;

    let res = client.upload(data, "<empty>", None).await?;
    let importer_code_id = res.code_id;
    info!(
        "  ✅ uploaded the importer contract in {}",
        res.transaction_hash
    );

    let res = client
        .instantiate(
            importer_code_id,
            &EmptyMessage {},
            "importer-contract".into(),
            "<empty>",
            Some(InstantiateOptions::default().with_admin(client.address())),
            None,
        )
        .await?;
    let importer_address = res.contract_address;
    info!(
        "  ✅ instantiated the importer contract in {}",
        res.transaction_hash
    );

    info!("IMPORTER CONTRACT ADDRESS: {importer_address}");

    CachedState {
        importer_address: importer_address.clone(),
        state_imported: false,
    }
    .save()?;

    Ok(importer_address)
}

async fn execute_prepare_contract(
    args: PrepareArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    create_importer_contract(args.importer_contract_path, &client).await?;

    Ok(())
}

fn approximate_size(pair: &Model) -> usize {
    base85::encode(&pair.key).len() + base85::encode(&pair.value).len()
}

fn models_to_exec(data: Vec<Model>) -> ExecuteMsg {
    let pairs = data
        .into_iter()
        .map(|kv| (kv.key, kv.value))
        .collect::<Vec<_>>();

    pairs.into()
}

fn split_into_importable_execute_msgs(
    kv_pairs: Vec<Model>,
    approximate_max_chunk: usize,
) -> Vec<ExecuteMsg> {
    let mut chunks: Vec<ExecuteMsg> = Vec::new();

    let mut current_wip_chunk = Vec::new();
    let mut current_chunk_size = 0;
    for kv in kv_pairs {
        if current_chunk_size + approximate_size(&kv) > approximate_max_chunk {
            let taken = std::mem::take(&mut current_wip_chunk);
            chunks.push(models_to_exec(taken));
            current_chunk_size = 0;
        }
        current_chunk_size += approximate_size(&kv);
        current_wip_chunk.push(kv);
    }

    if !current_wip_chunk.is_empty() {
        chunks.push(models_to_exec(current_wip_chunk))
    }
    chunks
}

async fn set_importer_state(
    state_dump_path: PathBuf,
    explicit_importer_address: Option<AccountId>,
    client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    info!("attempting to set the importer contract state");

    // this is the value that we found to be optimal during v1->v2 mixnet migration
    const MAX_CHUNK_SIZE: usize = 350 * 1000;

    let importer_address = importer_contract_address(explicit_importer_address)?;

    if let Ok(state) = CachedState::load() {
        if state.state_imported && state.importer_address == importer_address {
            bail!("the state has already been imported for {importer_address}")
        }
    }

    let dump_file = File::open(state_dump_path)?;
    info!("attempting to decode the state dump. for bigger contracts this might take a while...");
    let kv_pairs: Vec<Model> = serde_json::from_reader(&dump_file)?;

    info!("there are {} key-value pairs to import", kv_pairs.len());
    info!("attempting to split them into {MAX_CHUNK_SIZE}B chunks ExecuteMsgs...");

    let chunks = split_into_importable_execute_msgs(kv_pairs, MAX_CHUNK_SIZE);
    info!("obtained {} execute msgs", chunks.len());

    let total = chunks.len();
    for (i, msg) in chunks.into_iter().enumerate() {
        info!("executing message {}/{total}...", i + 1);
        let res = client
            .execute(
                &importer_address,
                &msg,
                None,
                "importing contract state",
                Vec::new(),
            )
            .await?;
        info!("  ✅ OK: {}", res.transaction_hash);
    }

    info!("Finished migrating storage to {importer_address}!");

    CachedState {
        importer_address,
        state_imported: true,
    }
    .save()?;

    Ok(())
}

async fn execute_set_state(
    args: SetStateArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    set_importer_state(args.raw_state, args.importer_contract_address, &client).await?;

    Ok(())
}

async fn swap_contract(
    target_code_id: Option<u64>,
    target_contract_path: Option<PathBuf>,
    explicit_importer_address: Option<AccountId>,
    migrate_msg: Option<serde_json::Value>,
    client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    info!("attempting to swap the contract code");

    let importer_address = importer_contract_address(explicit_importer_address)?;

    if let Ok(state) = CachedState::load() {
        if !state.state_imported && state.importer_address == importer_address {
            bail!("the state hasn't been imported for {importer_address}")
        }
    }

    // one of those must have been set via clap
    let code_id = match target_code_id {
        Some(explicit) => explicit,
        None => {
            // upload the contract
            let mut data = Vec::new();
            File::open(target_contract_path.unwrap())?.read_to_end(&mut data)?;

            let res = client.upload(data, "<empty>", None).await?;
            info!(
                "  ✅ uploaded the target contract in {}",
                res.transaction_hash
            );
            res.code_id
        }
    };

    let migrate_msg = migrate_msg.unwrap_or(serde_json::Value::Object(Default::default()));
    let res = client
        .migrate(
            &importer_address,
            code_id,
            &migrate_msg,
            "migrating into target contract",
            None,
        )
        .await?;
    info!(
        "  ✅ migrated into the target contract: {}",
        res.transaction_hash
    );

    Ok(())
}

async fn execute_swap_contract(
    args: SwapContractArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    swap_contract(
        args.target_contract_code_id,
        args.target_contract_path,
        args.importer_contract_address,
        args.migrate_msg,
        &client,
    )
    .await?;

    Ok(())
}

async fn initialise_with_state(
    args: InitialiseWithStateArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    let importer_address = create_importer_contract(args.importer_contract_path, &client).await?;
    set_importer_state(args.raw_state, Some(importer_address.clone()), &client).await?;
    swap_contract(
        args.target_contract_code_id,
        args.target_contract_path,
        Some(importer_address.clone()),
        args.migrate_msg,
        &client,
    )
    .await?;

    info!("the contract is ready at {importer_address}");

    Ok(())
}

impl Cli {
    pub async fn execute(self) -> anyhow::Result<()> {
        let network_details = NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;
        let nyxd_url = network_details
            .endpoints
            .first()
            .expect("network details are not defined")
            .nyxd_url
            .as_str();

        let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            nyxd_url,
            self.mnemonic,
        )?;
        match self.command {
            Commands::PrepareContract(args) => execute_prepare_contract(args, client).await,
            Commands::SetState(args) => execute_set_state(args, client).await,
            Commands::SwapContract(args) => execute_swap_contract(args, client).await,
            Commands::InitialiseWithState(args) => initialise_with_state(args, client).await,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_env(cli.config_env_file.as_ref());

    setup_tracing_logger();
    cli.execute().await
}
