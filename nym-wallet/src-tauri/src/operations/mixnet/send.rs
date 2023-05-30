use crate::error::BackendError;
use crate::state::WalletState;
use nym_types::currency::DecCoin;
use nym_types::transaction::{SendTxResult, TransactionDetails};
use nym_validator_client::nyxd::{AccountId, Fee};
use std::str::FromStr;

#[tauri::command]
pub async fn send(
    address: &str,
    amount: DecCoin,
    memo: String,
    fee: Option<Fee>,
    state: tauri::State<'_, WalletState>,
) -> Result<SendTxResult, BackendError> {
    let guard = state.read().await;
    let amount_base = guard.attempt_convert_to_base_coin(amount.clone())?;

    let to_address = AccountId::from_str(address)?;
    let from_address = guard.current_client()?.nyxd.address().to_string();
    let fee_amount = guard.convert_tx_fee(fee.as_ref());
    log::info!(
        ">>> Send: display_amount = {}, base_amount = {}, from = {}, to = {}, fee = {:?}",
        amount,
        amount_base,
        from_address,
        to_address,
        fee,
    );
    let raw_res = guard
        .current_client()?
        .nyxd
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
