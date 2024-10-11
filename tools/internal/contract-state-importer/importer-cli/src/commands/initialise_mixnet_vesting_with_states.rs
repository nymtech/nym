// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::prepare::create_importer_contract;
use crate::commands::set_state::set_importer_state;
use crate::commands::swap_contract::swap_contract;
use crate::helpers::{mixnet_contract_path, vesting_contract_path};
use clap::Args;
use importer_contract::ExecuteMsg;
use nym_mixnet_contract_common::{Addr, ContractState};
use nym_validator_client::nyxd::{AccountId, CosmWasmClient};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use serde::Serialize;
use std::path::PathBuf;
use tracing::info;

#[derive(Args, Clone)]
pub struct InitialiseMixnetVestingWithStatesArgs {
    /// Path to the .wasm file with the importer contract
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/importer_contract.wasm"
    #[clap(long)]
    pub importer_contract_path: Option<PathBuf>,

    /// Path to the file containing state dump of the mixnet contract
    #[clap(long)]
    pub mixnet_state: PathBuf,

    /// Path to the file containing state dump of the vesting contract
    #[clap(long)]
    pub vesting_state: PathBuf,

    /// Path to mixnet contract that will be uploaded and applied to the imported state
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/mixnet_contract.wasm"
    #[clap(long)]
    pub mixnet_contract_path: Option<PathBuf>,

    /// Path to the vesting contract that will be uploaded and applied to the imported state
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/vesting_contract.wasm"
    #[clap(long)]
    pub vesting_contract_path: Option<PathBuf>,
}

pub async fn update_kv<T>(
    client: &DirectSigningHttpRpcNyxdClient,
    contract: &AccountId,
    key: &[u8],
    value: &T,
) -> anyhow::Result<()>
where
    T: Serialize,
{
    let msg = ExecuteMsg::from(vec![(key.to_vec(), serde_json::to_vec(value)?)]);
    client
        .execute(contract, &msg, None, "updating kv...", Vec::new())
        .await?;
    Ok(())
}

#[allow(deprecated)]
pub async fn execute_initialise_mixnet_vesting_with_states(
    args: InitialiseMixnetVestingWithStatesArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    let mixnet_contract = mixnet_contract_path(args.mixnet_contract_path)?;
    let vesting_contract = vesting_contract_path(args.vesting_contract_path)?;

    info!(
        "using the following mixnet contract: {}",
        mixnet_contract.display()
    );
    info!(
        "using the following vesting contract: {}",
        vesting_contract.display()
    );

    let admin = client.address();

    // 1. import mixnet state
    info!("importing the mixnet contract state...");
    let mixnet_importer_address =
        create_importer_contract(args.importer_contract_path.clone(), &client).await?;
    set_importer_state(
        args.mixnet_state,
        Some(mixnet_importer_address.clone()),
        &client,
    )
    .await?;

    // 2. import vesting state
    info!("importing the vesting contract state...");
    let vesting_importer_address =
        create_importer_contract(args.importer_contract_path, &client).await?;
    set_importer_state(
        args.vesting_state,
        Some(vesting_importer_address.clone()),
        &client,
    )
    .await?;

    // 3. adjust few state entries
    // update vesting admin:
    info!("adjusting internal contract states...");
    let new_vesting_admin = Addr::unchecked(admin.as_ref());
    update_kv(
        &client,
        &vesting_importer_address,
        b"adm",
        &new_vesting_admin,
    )
    .await?;

    // update mixnet contract address in the vesting contract
    let new_mixnet_contract_address = Addr::unchecked(mixnet_importer_address.as_ref());
    update_kv(
        &client,
        &vesting_importer_address,
        b"mix",
        &new_mixnet_contract_address,
    )
    .await?;

    // update admins and vesting contract addresses in the mixnet contract
    let raw_contract_state = client
        .query_contract_raw(&mixnet_importer_address, b"state".to_vec())
        .await?;
    let mut contract_state: ContractState = serde_json::from_slice(&raw_contract_state)?;
    contract_state.vesting_contract_address = Addr::unchecked(vesting_importer_address.as_ref());
    contract_state.owner = Some(Addr::unchecked(admin.as_ref()));
    contract_state.rewarding_validator_address = Addr::unchecked(admin.as_ref());
    update_kv(&client, &mixnet_importer_address, b"state", &contract_state).await?;

    let contract_admin = Some(Addr::unchecked(admin.as_ref()));
    update_kv(&client, &mixnet_importer_address, b"admin", &contract_admin).await?;

    // 4. apply the correct contract codes
    info!("swapping the mixnet contract to the correct .wasm...");
    swap_contract(
        None,
        Some(mixnet_contract),
        Some(mixnet_importer_address.clone()),
        None,
        &client,
    )
    .await?;

    info!("swapping the vesting contract to the correct .wasm...");
    swap_contract(
        None,
        Some(vesting_contract),
        Some(vesting_importer_address.clone()),
        None,
        &client,
    )
    .await?;

    info!("the contracts are ready");
    info!("MIXNET: {mixnet_importer_address} VESTING: {vesting_importer_address}");

    Ok(())
}
