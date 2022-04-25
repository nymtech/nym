use crate::coin::{Coin, Denom};
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::Operation;
use mixnet_contract_common::mixnode::DelegationEvent as ContractDelegationEvent;
use mixnet_contract_common::mixnode::PendingUndelegate as ContractPendingUndelegate;
use mixnet_contract_common::Delegation;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(non_snake_case)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/appEnv.ts"))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AppEnv {
  pub ADMIN_ADDRESS: Option<String>,
  pub SHOW_TERMINAL: Option<String>,
}

fn get_env_as_option(key: &str) -> Option<String> {
  match ::std::env::var(key) {
    Ok(res) => Some(res),
    Err(_e) => None,
  }
}

#[tauri::command]
pub fn get_env() -> AppEnv {
  AppEnv {
    ADMIN_ADDRESS: get_env_as_option("ADMIN_ADDRESS"),
    SHOW_TERMINAL: get_env_as_option("SHOW_TERMINAL"),
  }
}

#[tauri::command]
pub fn major_to_minor(amount: &str) -> Coin {
  let coin = Coin::new(amount, &Denom::Major);
  coin.to_minor()
}

#[tauri::command]
pub fn minor_to_major(amount: &str) -> Coin {
  let coin = Coin::new(amount, &Denom::Minor);
  coin.to_major()
}

#[tauri::command]
pub async fn owns_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
  Ok(
    nymd_client!(state)
      .owns_mixnode(nymd_client!(state).address())
      .await?
      .is_some(),
  )
}

#[tauri::command]
pub async fn owns_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
  Ok(
    nymd_client!(state)
      .owns_gateway(nymd_client!(state).address())
      .await?
      .is_some(),
  )
}

// NOTE: this uses OUTDATED defaults that might have no resemblance with the reality
// as for the actual transaction, the gas cost is being simulated beforehand
#[tauri::command]
pub async fn outdated_get_approximate_fee(
  operation: Operation,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, BackendError> {
  let approximate_fee = operation.default_fee(nymd_client!(state).gas_price());
  let mut coin = Coin::new("0", &Denom::Major);
  for f in approximate_fee.amount {
    coin = coin + f.into();
  }
  Ok(coin)
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/delegationresult.ts"))]
#[derive(Serialize, Deserialize)]
pub struct DelegationResult {
  source_address: String,
  target_address: String,
  amount: Option<Coin>,
}

impl DelegationResult {
  pub fn new(source_address: &str, target_address: &str, amount: Option<Coin>) -> DelegationResult {
    DelegationResult {
      source_address: source_address.to_string(),
      target_address: target_address.to_string(),
      amount,
    }
  }
}

impl From<Delegation> for DelegationResult {
  fn from(delegation: Delegation) -> Self {
    DelegationResult {
      source_address: delegation.owner().to_string(),
      target_address: delegation.node_identity(),
      amount: Some(delegation.amount.into()),
    }
  }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/delegationevent.ts"))]
#[derive(Deserialize, Serialize)]
pub enum DelegationEvent {
  Delegate(DelegationResult),
  Undelegate(PendingUndelegate),
}

impl From<ContractDelegationEvent> for DelegationEvent {
  fn from(event: ContractDelegationEvent) -> Self {
    match event {
      ContractDelegationEvent::Delegate(delegation) => DelegationEvent::Delegate(delegation.into()),
      ContractDelegationEvent::Undelegate(pending_undelegate) => {
        DelegationEvent::Undelegate(pending_undelegate.into())
      }
    }
  }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/pendingundelegate.ts"))]
#[derive(Deserialize, Serialize)]
pub struct PendingUndelegate {
  mix_identity: String,
  delegate: String,
  proxy: Option<String>,
  block_height: u64,
}

impl From<ContractPendingUndelegate> for PendingUndelegate {
  fn from(pending_undelegate: ContractPendingUndelegate) -> Self {
    PendingUndelegate {
      mix_identity: pending_undelegate.mix_identity(),
      delegate: pending_undelegate.delegate().to_string(),
      proxy: pending_undelegate.proxy().map(|p| p.to_string()),
      block_height: pending_undelegate.block_height(),
    }
  }
}
