use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use nym_types::currency::DecCoin;
use nym_types::transaction::{SendTxResult, TransactionDetails};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{AccountId, Fee};

#[tauri::command]
pub async fn send(
    address: &str,
    amount: DecCoin,
    memo: String,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<SendTxResult, BackendError> {
    let guard = state.read().await;
    let amount_base = guard.attempt_convert_to_base_coin(amount.clone())?;

    let to_address = AccountId::from_str(address)?;
    let from_address = nymd_client!(state).address().to_string();
    let fee_amount = fee.as_ref().and_then(|fee| fee.try_get_manual_amount());
    log::info!(
        ">>> Send: display_amount = {}, base_amount = {}, from = {}, to = {}, fee = {:?}",
        amount,
        amount_base,
        from_address,
        to_address,
        fee,
    );
    let raw_res = nymd_client!(state)
        .send(&to_address, vec![amount_base], memo, fee)
        .await?;
    log::info!("<<< tx hash = {}", raw_res.hash.to_string());
    let res = SendTxResult::new(
        raw_res,
        TransactionDetails::new(amount, from_address, to_address.to_string()),
        fee_amount,
    );
    log::trace!("<<< {:?}", res);
    Ok(res)
}
