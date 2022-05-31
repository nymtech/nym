use std::{str::FromStr, sync::Arc};

use mixnet_contract_common::IdentityKey;
use tauri::async_runtime::RwLock;
use validator_client::nymd::{cosmwasm_client::types::ExecuteResult, error::NymdError, Fee};

use crate::{nymd_client, state::State};

#[tauri::command]
pub async fn claim_delegator_reward(
    identity: &str,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ExecuteResult, NymdError> {
    let identity_key = IdentityKey::from_str(identity).unwrap();
    Ok(nymd_client!(state)
        .claim_delegator_reward(identity_key, fee)
        .await
        .unwrap())
}
