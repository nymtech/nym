// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::nyxd_client;
use crate::state::WalletState;
use nym_types::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, PagedMixnetQueryClient};
use nym_wallet_types::interval::Interval;

#[tauri::command]
pub async fn get_current_interval(
    state: tauri::State<'_, WalletState>,
) -> Result<Interval, BackendError> {
    log::info!(">>> Get current interval");
    let res = nyxd_client!(state).get_current_interval_details().await?;
    log::info!("<<< current interval = {:?}", res);
    Ok(res.interval.into())
}

#[tauri::command]
pub async fn get_pending_epoch_events(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<PendingEpochEvent>, BackendError> {
    log::info!(">>> Get pending epoch events");
    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = guard.current_client()?;
    let res = client.nyxd.get_all_pending_epoch_events().await?;

    log::info!("<<< got = {:?} events", res.len());

    let converted = res
        .into_iter()
        .map(|e| PendingEpochEvent::try_from_mixnet_contract(e, reg))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(converted)
}

#[tauri::command]
pub async fn get_pending_interval_events(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<PendingIntervalEvent>, BackendError> {
    log::info!(">>> Get pending interval events");

    let guard = state.read().await;
    let reg = guard.registered_coins()?;
    let client = guard.current_client()?;
    let res = client.nyxd.get_all_pending_interval_events().await?;

    log::info!("<<< got = {:?} events", res.len());

    let converted = res
        .into_iter()
        .map(|e| PendingIntervalEvent::try_from_mixnet_contract(e, reg))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(converted)
}
