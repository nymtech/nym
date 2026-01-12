// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::DbPool;
use crate::db::queries::wasm::insert_wasm_execute;
use async_trait::async_trait;
use cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContract;
use cosmrs::proto::prost::Message;
use nym_validator_client::nyxd::{Any, Name};
use nyxd_scraper_psql::models::DbCoin;
use nyxd_scraper_psql::{
    MsgModule, NyxdScraperTransaction, ParsedTransactionResponse, ScraperError,
};
use serde_json::Value;
use time::{OffsetDateTime, PrimitiveDateTime};
use tracing::{error, trace};
use utoipa::r#gen::serde_json;

pub struct WasmModule {
    connection_pool: DbPool,
}

impl WasmModule {
    pub fn new(connection_pool: DbPool) -> Self {
        WasmModule { connection_pool }
    }
}

#[async_trait]
impl MsgModule for WasmModule {
    fn type_url(&self) -> String {
        MsgExecuteContract::type_url()
    }

    async fn handle_msg(
        &mut self,
        index: usize,
        msg: &Any,
        tx: &ParsedTransactionResponse,
        _storage_tx: &mut dyn NyxdScraperTransaction,
    ) -> Result<(), ScraperError> {
        let message = serde_json::to_value(tx.parsed_messages.get(&index)).unwrap_or_default();
        let value = serde_json::to_value(message.clone()).unwrap_or_default();
        let wasm_message_type = get_wasm_message_type(&value);
        let fee: Vec<DbCoin> = tx
            .tx
            .auth_info
            .fee
            .amount
            .clone()
            .into_iter()
            .map(|x| DbCoin {
                amount: x.amount.to_string(),
                denom: x.denom.to_string(),
            })
            .collect();

        let offset_datetime: OffsetDateTime = tx.block.header.time.into();
        let executed_at = PrimitiveDateTime::new(offset_datetime.date(), offset_datetime.time());

        let height = tx.height.value() as i64;
        let hash = tx.hash.to_string();
        let memo = tx.tx.body.memo.clone();

        match MsgExecuteContract::decode(msg.value.as_ref()) {
            Ok(wasm_execute_msg) => {
                let funds: Vec<DbCoin> = wasm_execute_msg
                    .funds
                    .clone()
                    .into_iter()
                    .map(|x| x.into())
                    .collect();
                let contract = wasm_execute_msg.contract;
                let sender = wasm_execute_msg.sender;

                if let Err(err) = insert_wasm_execute(
                    &self.connection_pool,
                    sender,
                    contract,
                    wasm_message_type,
                    &message,
                    Some(funds),
                    executed_at,
                    height,
                    hash,
                    index as i64,
                    memo,
                    &fee,
                )
                .await
                {
                    error!("Error persisting wasm contract execution message: {}", err);
                }
            }
            Err(err) => {
                error!("Error decoding message: {}", err);
            }
        }

        Ok(())
    }
}

fn get_first_field_name(value: Option<&Value>) -> Option<String> {
    trace!("value:\n{value:?}");
    match value {
        Some(value) => match value.as_object() {
            Some(map) => map.keys().next().cloned(),
            None => None,
        },
        None => None,
    }
}

fn get_wasm_message_type(value: &Value) -> String {
    get_first_field_name(value.get("msg")).unwrap_or_default()
}
