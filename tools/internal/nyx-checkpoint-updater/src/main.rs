// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{Context, Result, bail};
use clap::Parser;
use nym_bin_common::bin_info_owned;
use nym_bin_common::logging::setup_tracing_logger;
use nym_crypto::asymmetric::ed25519;
use nym_directory_client::anchor::checkpoint::{Checkpoint, SignedCheckpoint};
use nym_directory_client::anchor::{nyx_default_options, verify_checkpoint_advances_one_hop};
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd;
use nym_validator_client::nyxd::{
    Height, NymNetworkDetails, NyxdClient, TendermintPublicKey, TendermintRpcClient,
    TendermintValidatorInfo, ValidatorSet,
};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::mem;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::info;
use zeroize::Zeroizing;

/// The compiled-in checkpoint datum file, relative to the repo root. A static
/// `directory_checkpoint.rs` wrapper embeds it via `include_str!`, so the tool only ever rewrites
/// this JSON data file - never Rust source.
const CHECKPOINT_DATA_REL_PATH: &str =
    "common/network-defaults/src/mainnet/directory_checkpoint.json";

fn parse_rfc3339_time(raw: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(raw, &Rfc3339)
}

#[derive(clap::Parser)]
struct Args {
    #[clap(long, env = "NYM_CHECKPOINT_ROOT_ATTESTER_ED25519_BS58_PRIVATE_KEY")]
    root_attester_private_key: String,

    #[clap(long, env = "NYM_CHECKPOINT_TRUSTED_RPC")]
    rpc: String,

    #[clap(long, env = "NYM_CHECKPOINT_MINTED_AT_OVERRIDE", value_parser = parse_rfc3339_time)]
    minted_at: Option<OffsetDateTime>,

    /// Block height to anchor the checkpoint to.
    /// Defaults to the RPC's latest settled block (current height - 2)
    #[clap(long, env = "NYM_CHECKPOINT_HEIGHT")]
    height: Option<u32>,

    /// Root of the nym repository. The datum file is derived as
    /// `<repo-root>/common/network-defaults/src/mainnet/directory_checkpoint.json`. When omitted,
    /// the repo root is discovered by walking up from the current directory.
    #[clap(long, env = "NYM_REPO_ROOT", group = "checkpoint-location")]
    repo_root: Option<PathBuf>,

    /// Explicit path to the regenerated checkpoint datum (JSON) file. Overrides `--repo-root` and
    /// discovery.
    #[clap(long, env = "NYM_CHECKPOINT_OUT", group = "checkpoint-location")]
    out: Option<PathBuf>,
}

impl Debug for Args {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Args")
            .field("root_attester_private_key", &"<redacted>")
            .field("rpc", &self.rpc)
            .field("minted_at", &self.minted_at)
            .field("height", &self.height)
            .field("repo_root", &self.repo_root)
            .field("out", &self.out)
            .finish()
    }
}

impl Args {
    /// Resolve the datum file to regenerate: an explicit `--out` wins, then `--repo-root` joined with
    /// [`CHECKPOINT_DATA_REL_PATH`], otherwise discover the repo root at runtime.
    ///
    /// Runtime discovery (rather than a compile-time `CARGO_MANIFEST_DIR` offset) keeps the tool
    /// correct when the binary is compiled in one place and run from another - it only needs to be
    /// invoked from somewhere inside a nym checkout.
    fn resolve_checkpoint_data_path(&self) -> Result<PathBuf> {
        if let Some(out) = &self.out {
            return Ok(out.clone());
        }
        if let Some(root) = &self.repo_root {
            return Ok(root.join(CHECKPOINT_DATA_REL_PATH));
        }
        discover_checkpoint_data_path()
    }
}

/// Walk up from the current directory until a directory contains [`CHECKPOINT_DATA_REL_PATH`].
fn discover_checkpoint_data_path() -> Result<PathBuf> {
    let start = std::env::current_dir()?;
    for dir in start.ancestors() {
        let candidate = dir.join(CHECKPOINT_DATA_REL_PATH);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!(
        "could not locate {CHECKPOINT_DATA_REL_PATH} by walking up from {}; pass --repo-root or --out",
        start.display()
    )
}

/// Connect a read-only nyxd RPC client. Contracts, denoms and gas are irrelevant for fetching
/// blocks and validator sets, so an empty network config is fine.
fn connect_query_client(rpc: &str) -> Result<QueryHttpRpcNyxdClient> {
    let network_defaults = NymNetworkDetails::new_empty();
    let client_config = nyxd::Config::try_from_nym_network_details(&network_defaults)?;
    Ok(NyxdClient::connect(client_config, rpc)?)
}

/// Retrieve the checkpoint data (signed header + both validator sets) from the trusted chain RPC.
///
/// `height` pins the anchor block; `None` defaults to `latest - 2`, so both the checkpoint's own
/// `height + 1` validator set and the `height + 2` block the one-hop self-verify advances into are
/// already committed (see the self-verify step for why the hop needs `height + 2`).
#[allow(clippy::unwrap_used)]
async fn fetch_chain_checkpoint(
    client: &QueryHttpRpcNyxdClient,
    height: Option<u32>,
) -> Result<Checkpoint> {
    fn log_validator_set(set: &ValidatorSet) {
        fn format_validator(validator: &TendermintValidatorInfo) -> String {
            let name = validator.name.clone().unwrap_or("???".to_string());
            let key = match validator.pub_key {
                TendermintPublicKey::Ed25519(_) => "tendermint/PubKeyEd25519",
                TendermintPublicKey::Secp256k1(_) => "tendermint/PubKeySecp256k1",
                _ => "UNKNOWN",
            };

            format!(
                "{name}: {} power: {}, priority: {} (key type {key})",
                validator.address,
                validator.power,
                validator.proposer_priority.value(),
            )
        }

        info!(">> total voting power: {}", set.total_voting_power.value());
        if let Some(proposer) = &set.proposer {
            info!(">> PROPOSER: {}", format_validator(proposer));
            info!("----------")
        }
        for validator in &set.validators {
            info!(">> {}", format_validator(validator));
        }
    }

    let height = match height {
        Some(h) => h,
        None => (client.latest_block().await?.block.header.height.value() as u32) - 2,
    };
    if height <= 1 {
        bail!("attempted to fetch genesis block...");
    }

    info!("fetching checkpoint at height {height}...");
    let checkpoint = Checkpoint::fetch(client, Height::from(height)).await?;

    // SAFETY: unwraps are fine because we're not fetching genesis
    let header = &checkpoint.signed_header.header;
    let commit = &checkpoint.signed_header.commit;
    info!("✅ managed to fetch the checkpoint");
    info!("----- HEADER -----");
    info!(
        ">> version: block: {} app: {}",
        header.version.block, header.version.app
    );
    info!(">> chain_id: {}", header.chain_id);
    info!(">> height: {}", header.height);
    info!(">> time: {}", header.time);
    info!(">> last block id: {}", header.last_block_id.unwrap());
    info!(">> last commit_hash: {}", header.last_commit_hash.unwrap());
    info!(">> data hash: {}", header.data_hash.unwrap());
    info!(">> validators hash: {}", header.validators_hash);
    info!(">> next validators hash: {}", header.next_validators_hash);
    info!(">> consensus_hash: {}", header.consensus_hash);
    info!(">> app_hash: {}", header.app_hash);
    info!(
        ">> last results hash: {}",
        header.last_results_hash.unwrap()
    );
    info!(">> evidence hash: {}", header.evidence_hash.unwrap());
    info!(">> proposer address: {}", header.proposer_address);
    info!("----- END HEADER -----");
    info!("----- COMMIT -----");
    info!(">> height: {}", commit.height);
    info!(">> round: {}", commit.round);
    info!(">> block_id: {}", commit.block_id);
    info!(">> included signatures: {}", commit.signatures.len());
    info!("----- END COMMIT -----");

    info!(
        "----- VALIDATORS ({}) -----",
        checkpoint.validators.validators.len()
    );
    log_validator_set(&checkpoint.validators);
    info!("----- END VALIDATORS -----");

    info!(
        "----- NEXT VALIDATORS ({}) -----",
        checkpoint.next_validators.validators.len()
    );
    log_validator_set(&checkpoint.next_validators);
    info!("----- END NEXT VALIDATORS -----");

    Ok(checkpoint)
}

/// Regenerate the compiled-in checkpoint datum file from `signed`.
///
/// Writes the whole [`SignedCheckpoint`] (checkpoint + `created_at` + root signature) - the sibling
/// `directory_checkpoint.rs` wrapper embeds it via `include_str!`, and the hardcoded provider parses
/// it back as a `SignedCheckpoint` and verifies the root signature. Serializing only the inner
/// checkpoint would drop the signature and make the datum unparseable.
fn update_hardcoded_checkpoint(signed: &SignedCheckpoint, out: &Path) -> Result<()> {
    let output = File::create(out).context("failed to open output file")?;
    serde_json::to_writer_pretty(output, signed).context("failed to serialize signed checkpoint")
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing_logger();

    let bin_info = bin_info_owned!();
    info!("using the following version: {bin_info}");

    let mut args = Args::parse();
    let key_str = Zeroizing::new(mem::take(&mut args.root_attester_private_key));
    let key = ed25519::PrivateKey::from_base58_string(&key_str)?;

    let out = args.resolve_checkpoint_data_path()?;
    let client = connect_query_client(&args.rpc)?;

    // 1. retrieve the checkpoint data from the trusted chain RPC
    let checkpoint = fetch_chain_checkpoint(&client, args.height).await?;

    // 2. sign it under the root attester key (advisory mint timestamp, defaults to now)
    let minted_at = args.minted_at.unwrap_or_else(OffsetDateTime::now_utc);
    let signed = SignedCheckpoint::new(checkpoint, minted_at, &key);

    // 3. self-verify before persisting: advance the minted checkpoint one light-client hop against
    // the same RPC (no anchor / contract needed). A malformed or non-chaining checkpoint fails here
    // and is never written.
    verify_checkpoint_advances_one_hop(&client, &signed.checkpoint, &nyx_default_options())
        .await
        .context(
            "self-verify failed: the minted checkpoint could not advance one light-client hop",
        )?;
    info!("✅ self-verify passed: checkpoint advances one light-client hop");

    // 4. regenerate the compiled-in constant file
    update_hardcoded_checkpoint(&signed, &out)?;

    info!("wrote regenerated checkpoint constant to {}", out.display());
    Ok(())
}
