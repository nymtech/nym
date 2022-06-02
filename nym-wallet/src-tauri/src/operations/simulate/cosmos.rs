// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coin::Coin;
use crate::error::BackendError;
use crate::operations::simulate::{FeeDetails, SimulateResult};
use crate::state::State;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{AccountId, MsgSend};

#[tauri::command]
pub async fn simulate_send(
    address: &str,
    amount: Coin,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<FeeDetails, BackendError> {
    let guard = state.read().await;

    let to_address = AccountId::from_str(address)?;
    let amount = amount.into_backend_coin(guard.current_network().denom())?;

    let client = guard.current_client()?;
    let from_address = client.nymd.address().clone();
    let gas_price = client.nymd.gas_price().clone();

    // TODO: I'm still not 100% convinced whether this should be exposed here or handled somewhere else in the client code
    let msg = MsgSend {
        from_address,
        to_address,
        amount: vec![amount.into()],
    };

    let result = client.nymd.simulate(vec![msg]).await?;
    Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
}
