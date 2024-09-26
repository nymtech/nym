// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use base64::Engine;
use cosmrs::abci::TxMsgData;
use cosmrs::cosmwasm::MsgExecuteContractResponse;
use cosmrs::proto::cosmos::base::query::v1beta1::{PageRequest, PageResponse};
use log::error;
use prost::bytes::Bytes;
use tendermint_rpc::endpoint::broadcast;

pub use cosmrs::abci::MsgResponse;

pub fn parse_msg_responses(data: Bytes) -> Vec<MsgResponse> {
    // it seems that currently, on wasmd 0.43 + tendermint-rs 0.37 + cosmrs 0.17.0-pre
    // the data is left in undecoded base64 form, but I'd imagine this might change so if the decoding fails,
    // use the bytes directly instead
    let data = if let Ok(decoded) = base64::prelude::BASE64_STANDARD.decode(&data) {
        decoded
    } else {
        error!("failed to base64-decode the 'data' field of the TxResponse - has the chain been upgraded and introduced some breaking changes?");
        data.into()
    };

    match TxMsgData::try_from(data) {
        Ok(tx_msg_data) => tx_msg_data.msg_responses,
        Err(err) => {
            error!("failed to parse tx responses - has the chain been upgraded and introduced some breaking changes? the error was {err}");
            Vec::new()
        }
    }
}

// requires there's a single response message
pub trait ToSingletonContractData: Sized {
    fn parse_singleton_u32_contract_data(&self) -> Result<u32, NyxdError> {
        let b = self.to_singleton_contract_data()?;
        if b.len() != 4 {
            return Err(NyxdError::MalformedResponseData {
                got: b.len(),
                expected: 4,
            });
        }
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn parse_singleton_u64_contract_data(&self) -> Result<u64, NyxdError> {
        let b = self.to_singleton_contract_data()?;
        if b.len() != 8 {
            return Err(NyxdError::MalformedResponseData {
                got: b.len(),
                expected: 8,
            });
        }
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn to_singleton_contract_data(&self) -> Result<Vec<u8>, NyxdError>;
}

impl ToSingletonContractData for ExecuteResult {
    fn to_singleton_contract_data(&self) -> Result<Vec<u8>, NyxdError> {
        if self.msg_responses.len() != 1 {
            return Err(NyxdError::UnexpectedNumberOfMsgResponses {
                got: self.msg_responses.len(),
            });
        }

        self.msg_responses[0].to_contract_response_data()
    }
}

pub trait ToContractResponseData: Sized {
    fn to_contract_response_data(&self) -> Result<Vec<u8>, NyxdError>;
}

impl ToContractResponseData for MsgResponse {
    fn to_contract_response_data(&self) -> Result<Vec<u8>, NyxdError> {
        Ok(self.try_decode_as::<MsgExecuteContractResponse>()?.data)
    }
}

pub(crate) trait CheckResponse: Sized {
    fn check_response(self) -> Result<Self, NyxdError>;
}

impl CheckResponse for broadcast::tx_commit::Response {
    fn check_response(self) -> Result<Self, NyxdError> {
        if self.check_tx.code.is_err() {
            return Err(NyxdError::BroadcastTxErrorCheckTx {
                hash: self.hash,
                height: Some(self.height),
                code: self.check_tx.code.value(),
                raw_log: self.check_tx.log,
            });
        }

        if self.tx_result.code.is_err() {
            return Err(NyxdError::BroadcastTxErrorDeliverTx {
                hash: self.hash,
                height: Some(self.height),
                code: self.tx_result.code.value(),
                raw_log: self.tx_result.log,
            });
        }

        Ok(self)
    }
}

impl CheckResponse for crate::nyxd::TxResponse {
    fn check_response(self) -> Result<Self, NyxdError> {
        if self.tx_result.code.is_err() {
            return Err(NyxdError::BroadcastTxErrorDeliverTx {
                hash: self.hash,
                height: Some(self.height),
                code: self.tx_result.code.value(),
                raw_log: self.tx_result.log,
            });
        }

        Ok(self)
    }
}

pub(crate) fn compress_wasm_code(code: &[u8]) -> Result<Vec<u8>, NyxdError> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    // using compression level 9, same as cosmjs, that optimises for size
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(code)
        .map_err(NyxdError::WasmCompressionError)?;
    encoder.finish().map_err(NyxdError::WasmCompressionError)
}

pub(crate) fn create_pagination(key: Vec<u8>) -> PageRequest {
    PageRequest {
        key,
        offset: 0,
        limit: 0,
        count_total: false,
        reverse: false,
    }
}

pub(crate) fn next_page_key(pagination_info: Option<PageResponse>) -> Option<Vec<u8>> {
    if let Some(next_page_info) = pagination_info {
        // it turns out, even though `PageResponse` is always returned wrapped in an `Option`,
        // the `next_key` can still be empty, so check whether we actually need to perform another call
        if !next_page_info.next_key.is_empty() {
            return Some(next_page_info.next_key);
        }
    }

    None
}
