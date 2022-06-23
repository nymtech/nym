use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use nym_types::currency::MajorCurrencyAmount;
use nym_types::transaction::{SendTxResult, TransactionDetails};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{AccountId, Fee};

#[tauri::command]
pub async fn send(
    address: &str,
    amount: MajorCurrencyAmount,
    memo: String,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<SendTxResult, BackendError> {
    let denom_minor = state.read().await.current_network().base_mix_denom();
    let address = AccountId::from_str(address)?;
    let from_address = nymd_client!(state).address().to_string();
    let amount2 = amount.clone().into();
    log::info!(
        ">>> Send: amount = {}, minor_amount = {:?}, from = {}, to = {}, fee = {:?}",
        amount,
        amount2,
        from_address,
        address.as_ref(),
        fee,
    );
    let raw_res = nymd_client!(state)
        .send(&address, vec![amount2], memo, fee)
        .await?;
    log::info!("<<< tx hash = {}", raw_res.hash.to_string());
    let res = SendTxResult::new(
        raw_res,
        TransactionDetails {
            from_address,
            to_address: address.to_string(),
            amount,
        },
        denom_minor.as_ref(),
    )?;
    log::trace!("<<< {:?}", res);
    Ok(res)
}
