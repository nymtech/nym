use crate::currency::MajorCurrencyAmount;
use crate::error::TypesError;
use crate::gas::GasInfo;
use serde::{Deserialize, Serialize};
use validator_client::nymd::cosmwasm_client::types::ExecuteResult;
use validator_client::nymd::TxResponse;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/SendTxResult.ts")
)]
#[derive(Deserialize, Serialize, Debug)]
pub struct SendTxResult {
    pub block_height: u64,
    pub code: u32,
    pub details: TransactionDetails,
    pub gas_used: u64,
    pub gas_wanted: u64,
    pub tx_hash: String,
    // pub fee: MajorCurrencyAmount,
}

impl SendTxResult {
    pub fn new(
        t: TxResponse,
        details: TransactionDetails,
        _denom_minor: &str,
    ) -> Result<SendTxResult, TypesError> {
        Ok(SendTxResult {
            block_height: t.height.value(),
            code: t.tx_result.code.value(),
            details,
            gas_used: t.tx_result.gas_used.value(),
            gas_wanted: t.tx_result.gas_wanted.value(),
            tx_hash: t.hash.to_string(),
            // that is completely wrong: fee is what you told the validator to use beforehand
            // fee: MajorCurrencyAmount::from_decimal_and_denom(
            //     Decimal::new(Uint128::from(t.tx_result.gas_used.value())),
            //     denom_minor.to_string(),
            // )?,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/TransactionDetails.ts")
)]
#[derive(Deserialize, Serialize, Debug)]
pub struct TransactionDetails {
    pub amount: MajorCurrencyAmount,
    pub from_address: String,
    pub to_address: String,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/TransactionExecuteResult.ts")
)]
#[derive(Deserialize, Serialize)]
pub struct TransactionExecuteResult {
    pub logs_json: String,
    pub data_json: String,
    pub transaction_hash: String,
    pub gas_info: GasInfo,
    pub fee: MajorCurrencyAmount,
}

impl TransactionExecuteResult {
    pub fn from_execute_result(
        value: ExecuteResult,
        denom_minor: &str,
    ) -> Result<TransactionExecuteResult, TypesError> {
        let gas_info = GasInfo::from_validator_client_gas_info(value.gas_info, denom_minor)?;
        let fee = gas_info.fee.clone();
        Ok(TransactionExecuteResult {
            gas_info,
            transaction_hash: value.transaction_hash.to_string(),
            data_json: ::serde_json::to_string_pretty(&value.data)?,
            logs_json: ::serde_json::to_string_pretty(&value.logs)?,
            fee,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/RpcTransactionResponse.ts")
)]
#[derive(Deserialize, Serialize)]
pub struct RpcTransactionResponse {
    pub index: u32,
    pub tx_result_json: String,
    pub block_height: u64,
    pub transaction_hash: String,
    pub gas_info: GasInfo,
    // pub fee: MajorCurrencyAmount,
}

impl RpcTransactionResponse {
    pub fn from_tx_response(
        value: &TxResponse,
        denom_minor: &str,
    ) -> Result<RpcTransactionResponse, TypesError> {
        Ok(RpcTransactionResponse {
            index: value.index,
            gas_info: GasInfo::from_u64(
                value.tx_result.gas_wanted.value(),
                value.tx_result.gas_used.value(),
                denom_minor,
            )?,
            transaction_hash: value.hash.to_string(),
            tx_result_json: ::serde_json::to_string_pretty(&value.tx_result)?,
            block_height: value.height.value(),
            // wrong
            // fee: MajorCurrencyAmount::from_decimal_and_denom(
            //     Decimal::new(Uint128::from(value.tx_result.gas_used.value())),
            //     denom_minor.to_string(),
            // )?,
        })
    }
}
