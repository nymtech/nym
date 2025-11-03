// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{CI_BUILD_SERVER, NYM_API_UTILITY_BEARER, contract_build_names};
use anyhow::{Context, bail};
use bytes::Buf;
use futures::stream::StreamExt;
use indicatif::ProgressBar;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::env::current_dir;
use std::ffi::{OsStr, OsString};
use std::fs::create_dir_all;
use std::future::Future;
use std::io::{BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;
use std::{fs, io};
use tokio::pin;
use tokio::process::Command;
use tokio::time::interval;
use tracing::{debug, error};
use url::Url;

pub(crate) async fn async_with_progress<F, T>(fut: F, pb: &ProgressBar) -> T
where
    F: Future<Output = T>,
{
    pb.tick();
    pin!(fut);
    let mut update_interval = interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            _ = update_interval.tick() => {
                pb.tick()
            }
            res = &mut fut => {
                return res
            }
        }
    }
}

pub(crate) fn wasm_code<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<u8>> {
    let path = path.as_ref();
    assert!(path.exists());
    let mut file = std::fs::File::open(path).context("failed to open wasm code")?;
    let mut data = Vec::new();

    file.read_to_end(&mut data)
        .context("failed to read wasm code")?;
    Ok(data)
}

pub(crate) async fn download_cosmwasm_contract(
    output_directory: impl AsRef<Path>,
    ci_build_branch: &str,
    contract_filename: &str,
) -> anyhow::Result<()> {
    let output_directory = output_directory.as_ref();
    let download_target = output_directory.join(contract_filename);

    create_dir_all(output_directory)?;

    let download_url = format!("{CI_BUILD_SERVER}/{ci_build_branch}/{contract_filename}");
    let response = reqwest::get(download_url).await?;

    let mut source = response.bytes_stream();

    let output_binary = fs::File::create(download_target)?;
    let mut out = BufWriter::new(output_binary);

    while let Some(chunk) = source.next().await {
        let mut bytes = chunk?.reader();
        io::copy(&mut bytes, &mut out)?;
    }

    Ok(())
}

/// Does not explicitly return an `Err` for exit code != 0
pub(crate) async fn exec_fallible_cmd_with_output<S1, S2, I>(
    cmd: S1,
    args: I,
) -> anyhow::Result<Output>
where
    I: IntoIterator<Item = S2>,
    S1: AsRef<OsStr>,
    S2: AsRef<OsStr>,
{
    let (cmd, cmd_args) = debug_args(cmd, args);

    let output = Command::new(cmd.clone())
        .args(cmd_args.clone())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?
        .wait_with_output()
        .await
        .inspect_err(|err| error!("{cmd:?} {cmd_args:?} FAILED WITH {err}"))?;

    Ok(output)
}

pub(crate) async fn exec_inherit_output<S1, S2, I>(cmd: S1, args: I) -> anyhow::Result<Output>
where
    I: IntoIterator<Item = S2>,
    S1: AsRef<OsStr>,
    S2: AsRef<OsStr>,
{
    let (cmd, cmd_args) = debug_args(cmd, args);

    let output = Command::new(cmd.clone())
        .args(cmd_args.clone())
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?
        .wait_with_output()
        .await
        .inspect_err(|err| error!("{cmd:?} {cmd_args:?} FAILED WITH {err}"))?;

    Ok(output)
}

/// Does explicitly return an `Err` for exit code != 0
pub(crate) async fn exec_cmd_with_output<S1, S2, I>(cmd: S1, args: I) -> anyhow::Result<Output>
where
    I: IntoIterator<Item = S2>,
    S1: AsRef<OsStr>,
    S2: AsRef<OsStr>,
{
    let cmd = cmd.as_ref();
    let output = exec_fallible_cmd_with_output(cmd, args).await?;

    if !output.status.success() {
        error!(
            "'{}' exited with status {}",
            cmd.to_string_lossy(),
            output.status
        );
        if !output.stderr.is_empty() {
            error!("{}", String::from_utf8_lossy(&output.stderr));
        }

        bail!(
            "'{}' exited with status {}",
            cmd.to_string_lossy(),
            output.status
        );
    }
    Ok(output)
}

pub(crate) fn generate_network_name() -> String {
    let mut rng = thread_rng();

    let words = bip39::Language::English.word_list();
    // SAFETY: this list is not empty
    #[allow(clippy::unwrap_used)]
    let first = words.choose(&mut rng).unwrap();
    #[allow(clippy::unwrap_used)]
    let second = words.choose(&mut rng).unwrap();
    format!("{first}-{second}")
}

// ordering doesn't matter for the purposes of this function
pub(crate) fn nym_cosmwasm_contract_names() -> Vec<&'static str> {
    vec![
        contract_build_names::MIXNET,
        contract_build_names::VESTING,
        contract_build_names::ECASH,
        contract_build_names::DKG,
        contract_build_names::GROUP,
        contract_build_names::MULTISIG,
        contract_build_names::PERFORMANCE,
    ]
}

// this is beyond hacky, but it works, for now*
// and is easier than attempting to retrieve the data from a running node
// (and node can't run without mixnet contract, for which we need the version)
// *assuming nym-node image is built from fresh
pub(crate) fn retrieve_current_nymnode_version<P: AsRef<Path>>(
    monorepo_root: P,
) -> anyhow::Result<String> {
    let nym_node_cargo_toml = monorepo_root.as_ref().join("nym-node/Cargo.toml");
    let manifest = cargo_edit::LocalManifest::find(Some(&nym_node_cargo_toml))?;
    Ok(manifest
        .data
        .get("package")
        .context("malformed nym-node Cargo.toml file - no 'package' section")?
        .get("version")
        .context("malformed nym-node Cargo.toml file - no [package].version set")?
        .as_str()
        .context("malformed nym-node Cargo.toml file - [package].version is not a string!")?
        .to_string())
}

pub(crate) fn monorepo_root_path(arg: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let maybe_path = match arg {
        Some(path) => path,
        None => {
            // ASSUMPTION: we're being run from the root of the nym repo
            current_dir()?
        }
    };

    if !maybe_path.exists() {
        bail!("'{}' does not exist", maybe_path.display());
    }

    let maybe_path_canon = maybe_path.canonicalize()?;

    // don't allow such degenerative cases
    let dir = maybe_path_canon
        .components()
        .next_back()
        .context("attempted to execute orchestrator from the root of the filesystem")?;
    if dir.as_os_str().to_string_lossy() != "nym" {
        bail!(
            "localnet-orchestrator must be executed from the root of the nym repo! the path is {maybe_path_canon:?}"
        );
    }

    Ok(maybe_path)
}

pub(crate) fn nym_api_cache_refresh_script(
    cache_timestamp_route: Url,
    cache_refresh_route: Url,
) -> String {
    // I prefer inlining the scripts over putting them in dedicated files (for the localnet purpose)
    // to the better flexibility in being able to modify them more easily
    format!(
        r#"
set -euo pipefail

# initial ts
initial_ts=$(curl --fail-with-body -s \
  -H "Authorization: Bearer {NYM_API_UTILITY_BEARER}" \
  {cache_timestamp_route} | jq -r '.timestamp')

# refresh cache
curl --fail-with-body -s -X POST {cache_refresh_route} \
  -H "Authorization: Bearer {NYM_API_UTILITY_BEARER}" \
  -H "Content-Type: application/json" \
  -d '{{}}' > /dev/null

# wait for the cache to actually get refreshed
while true; do
    current_ts=$(curl --fail-with-body -s \
      -H "Authorization: Bearer {NYM_API_UTILITY_BEARER}" \
      {cache_timestamp_route} | jq -r '.timestamp')

    if [ "$(date -d "$current_ts" +%s%N)" -gt "$(date -d "$initial_ts" +%s%N)" ]; then
        break
    fi

    sleep 0.2
done
        "#,
    )
}

fn debug_args<S1, S2, I>(cmd: S1, args: I) -> (OsString, Vec<OsString>)
where
    I: IntoIterator<Item = S2>,
    S1: AsRef<OsStr>,
    S2: AsRef<OsStr>,
{
    let mut cmd_args = Vec::new();
    let mut args_debug = Vec::new();
    for arg in args {
        let arg = arg.as_ref();
        args_debug.push(arg.to_string_lossy().to_string());
        cmd_args.push(arg.to_os_string());
    }

    let cmd = cmd.as_ref().to_os_string();
    let cmd_debug = cmd.to_string_lossy();

    debug!("executing: {cmd_debug} {}", args_debug.join(" "));

    (cmd, cmd_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::cosmwasm_contract::ContractBeingInitialised;
    use crate::orchestrator::network::NymContractsBeingInitialised;

    #[test]
    fn all_contracts_are_included() {
        let contracts = NymContractsBeingInitialised {
            mixnet: ContractBeingInitialised::new("mixnet"),
            vesting: ContractBeingInitialised::new("vesting"),
            ecash: ContractBeingInitialised::new("ecash"),
            cw3_multisig: ContractBeingInitialised::new("cw3-multisig"),
            cw4_group: ContractBeingInitialised::new("cw4-group"),
            dkg: ContractBeingInitialised::new("dkg"),
            performance: ContractBeingInitialised::new("performance"),
        };

        assert_eq!(
            nym_cosmwasm_contract_names().len(),
            NymContractsBeingInitialised::COUNT
        );
    }
}
