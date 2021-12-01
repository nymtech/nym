use crate::coin::Coin;
use crate::format_err;
use crate::state::State;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use tendermint_rpc::endpoint::broadcast::tx_commit::Response;
use tokio::sync::RwLock;
use validator_client::nymd::{AccountId, CosmosCoin};

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Deserialize, Serialize)]
pub struct TauriTxResult {
  block_height: u64,
  code: u32,
  details: TransactionDetails,
  gas_used: u64,
  gas_wanted: u64,
  tx_hash: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Deserialize, Serialize)]
pub struct TransactionDetails {
  amount: Coin,
  from_address: String,
  to_address: String,
}

impl TauriTxResult {
  fn new(t: Response, details: TransactionDetails) -> TauriTxResult {
    TauriTxResult {
      block_height: t.height.value(),
      code: t.check_tx.code.value(),
      details,
      gas_used: t.check_tx.gas_used.value(),
      gas_wanted: t.check_tx.gas_wanted.value(),
      tx_hash: t.hash.to_string(),
    }
  }
}

#[tauri::command]
pub async fn send(
  address: &str,
  amount: Coin,
  memo: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriTxResult, String> {
  let address = match AccountId::from_str(address) {
    Ok(addy) => addy,
    Err(e) => return Err(format_err!(e)),
  };
  let cosmos_amount: CosmosCoin = match amount.clone().try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.send(&address, vec![cosmos_amount], memo).await {
    Ok(result) => Ok(TauriTxResult::new(
      result,
      TransactionDetails {
        from_address: client.address().to_string(),
        to_address: address.to_string(),
        amount,
      },
    )),
    Err(e) => Err(format_err!(e)),
  }
}
