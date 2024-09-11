// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::nyxd_client;
use crate::state::WalletState;
use nym_mixnet_contract_common::ExecuteMsg;
use nym_types::transaction::TransactionExecuteResult;
use nym_validator_client::nyxd::contract_traits::{
    MixnetSigningClient, NymContractsProvider, PagedMixnetQueryClient,
};
use nym_validator_client::nyxd::Fee;

#[tauri::command]
pub async fn migrate_vested_mixnode(
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    let guard = state.read().await;
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(">>> migrate vested mixnode, fee = {fee:?}");

    let res = nyxd_client!(state).migrate_vested_mixnode(fee).await?;
    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(
        res, fee_amount,
    )?)
}

#[tauri::command]
pub async fn migrate_vested_delegations(
    state: tauri::State<'_, WalletState>,
) -> Result<TransactionExecuteResult, BackendError> {
    log::info!(">>> migrate vested delegations");

    let guard = state.read().await;
    let client = guard.current_client()?;

    let address = client.nyxd.address();
    let mixnet_contract = client
        .nyxd
        .mixnet_contract_address()
        .expect("unavailable mixnet contract address");

    log::info!("  >>> Get delegations");
    let delegations = client
        .nyxd
        .get_all_delegator_delegations(&address)
        .await
        .inspect_err(|err| {
            log::error!("  <<< Failed to get delegations. Error: {}", err);
        })?;
    log::info!("  <<< {} delegations", delegations.len());

    let vesting_delegations = delegations
        .into_iter()
        .filter(|d| d.proxy.is_some())
        .collect::<Vec<_>>();

    log::info!("  <<< {} vesting delegations", vesting_delegations.len());

    if vesting_delegations.is_empty() {
        return Err(BackendError::NoVestingDelegations);
    }

    let mut migrate_msgs = Vec::new();
    for delegation in &vesting_delegations {
        migrate_msgs.push((
            ExecuteMsg::MigrateVestedDelegation {
                mix_id: delegation.mix_id,
            },
            Vec::new(),
        ));
    }

    let res = client
        .nyxd
        .execute_multiple(
            mixnet_contract,
            migrate_msgs,
            None,
            format!(
                "migrating {} vesting delegations",
                vesting_delegations.len()
            ),
        )
        .await?;

    log::info!("<<< tx hash = {}", res.transaction_hash);
    log::trace!("<<< {:?}", res);
    Ok(TransactionExecuteResult::from_execute_result(res, None)?)
}
