use crate::currency::DecCoin;
use crate::error::TypesError;
use crate::gas::{Gas, GasInfo};
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::TxResponse;
use serde::{Deserialize, Serialize};

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
    pub gas_used: Gas,
    pub gas_wanted: Gas,
    pub tx_hash: String,
    pub fee: Option<DecCoin>,
}

impl SendTxResult {
    pub fn new(t: TxResponse, details: TransactionDetails, fee: Option<DecCoin>) -> SendTxResult {
        SendTxResult {
            block_height: t.height.value(),
            code: t.tx_result.code.value(),
            details,
            gas_used: t.tx_result.gas_used.into(),
            gas_wanted: t.tx_result.gas_wanted.into(),
            tx_hash: t.hash.to_string(),
            fee,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/TransactionDetails.ts")
)]
#[derive(Deserialize, Serialize, Debug)]
pub struct TransactionDetails {
    pub amount: DecCoin,
    pub from_address: String,
    pub to_address: String,
}

impl TransactionDetails {
    pub fn new(amount: DecCoin, from_address: String, to_address: String) -> Self {
        TransactionDetails {
            amount,
            from_address,
            to_address,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/TransactionExecuteResult.ts")
)]
#[derive(Deserialize, Serialize, Debug)]
pub struct TransactionExecuteResult {
    pub logs_json: String,
    pub msg_responses_json: String,
    pub transaction_hash: String,
    pub gas_info: GasInfo,
    pub fee: Option<DecCoin>,
}

impl TransactionExecuteResult {
    pub fn from_execute_result(
        value: ExecuteResult,
        fee: Option<DecCoin>,
    ) -> Result<TransactionExecuteResult, TypesError> {
        Ok(TransactionExecuteResult {
            gas_info: value.gas_info.into(),
            transaction_hash: value.transaction_hash.to_string(),
            msg_responses_json: ::serde_json::to_string_pretty(&value.msg_responses)?,
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
    pub gas_used: Gas,
    pub gas_wanted: Gas,
    pub fee: Option<DecCoin>,
}

impl RpcTransactionResponse {
    pub fn from_tx_response(
        t: &TxResponse,
        fee: Option<DecCoin>,
    ) -> Result<RpcTransactionResponse, TypesError> {
        Ok(RpcTransactionResponse {
            index: t.index,
            gas_used: t.tx_result.gas_used.into(),
            gas_wanted: t.tx_result.gas_wanted.into(),
            transaction_hash: t.hash.to_string(),
            tx_result_json: ::serde_json::to_string_pretty(&t.tx_result)?,
            block_height: t.height.value(),
            fee,
        })
    }
}
