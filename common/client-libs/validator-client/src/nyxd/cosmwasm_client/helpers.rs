// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use base64::Engine;
use cosmrs::abci::TxMsgData;
use cosmrs::cosmwasm::MsgExecuteContractResponse;
use cosmrs::AccountId;
use cosmwasm_std::from_json;
use prost::bytes::Bytes;
use serde::de::DeserializeOwned;
use tendermint_rpc::endpoint::broadcast;
use tracing::error;

pub use cosmrs::abci::MsgResponse;

/// Build the raw `x/wasm` store key for a contract's storage entry:
/// `ContractStorePrefix (0x03) || contract_address_bytes || contract_key`.
///
/// This is the multistore-relative key (queried under `store/wasm/key`) that an
/// `abci_query` membership proof commits to, so an off-chain verifier can reconstruct
/// it independently of the RPC response. See
/// <https://github.com/CosmWasm/wasmd/blob/v0.60.0/x/wasm/types/keys.go#L30> and
/// <https://github.com/CosmWasm/wasmd/blob/v0.60.0/x/wasm/keeper/keeper.go#L924-L926>.
pub fn contract_storage_key(contract: &AccountId, contract_key: &[u8]) -> Vec<u8> {
    // 0x03 is the wasmd 'ContractStorePrefix' constant
    const CONTRACT_STORE_PREFIX: u8 = 0x03;

    let addr = contract.to_bytes();
    let mut key = Vec::with_capacity(1 + addr.len() + contract_key.len());
    key.push(CONTRACT_STORE_PREFIX);
    key.extend_from_slice(&addr);
    key.extend_from_slice(contract_key);
    key
}

pub fn parse_singleton_u32_from_contract_response(b: Vec<u8>) -> Result<u32, NyxdError> {
    if b.len() != 4 {
        return Err(NyxdError::MalformedResponseData {
            got: b.len(),
            expected: 4,
        });
    }
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn parse_singleton_u64_from_contract_response(b: Vec<u8>) -> Result<u64, NyxdError> {
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

#[derive(Debug, Clone)]
pub struct ParsedContractResponse {
    pub message_index: usize,
    pub response: Vec<u8>,
}

impl ParsedContractResponse {
    pub fn parse_singleton_u32_contract_data(self) -> Result<u32, NyxdError> {
        parse_singleton_u32_from_contract_response(self.response)
    }

    pub fn parse_singleton_u64_contract_data(self) -> Result<u64, NyxdError> {
        parse_singleton_u64_from_contract_response(self.response)
    }
}

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
pub trait ContractResponseData: Sized {
    fn parse_singleton_json_contract_response<T: DeserializeOwned>(&self) -> Result<T, NyxdError> {
        let b = self.to_singleton_contract_data()?;
        from_json(&b).map_err(|err| err.into())
    }

    fn parse_singleton_u32_contract_data(&self) -> Result<u32, NyxdError> {
        let b = self.to_singleton_contract_data()?;
        parse_singleton_u32_from_contract_response(b)
    }

    fn parse_singleton_u64_contract_data(&self) -> Result<u64, NyxdError> {
        let b = self.to_singleton_contract_data()?;
        parse_singleton_u64_from_contract_response(b)
    }

    fn to_singleton_contract_data(&self) -> Result<Vec<u8>, NyxdError>;

    fn to_unchecked_contract_data(&self) -> Result<Vec<Vec<u8>>, NyxdError>;

    fn to_contract_data(&self) -> Result<Vec<ParsedContractResponse>, NyxdError>;
}

impl ContractResponseData for ExecuteResult {
    fn to_singleton_contract_data(&self) -> Result<Vec<u8>, NyxdError> {
        if self.msg_responses.len() != 1 {
            return Err(NyxdError::UnexpectedNumberOfMsgResponses {
                got: self.msg_responses.len(),
            });
        }

        self.msg_responses[0].to_contract_response_data()
    }

    fn to_unchecked_contract_data(&self) -> Result<Vec<Vec<u8>>, NyxdError> {
        self.msg_responses
            .iter()
            .map(ToContractResponseData::to_contract_response_data)
            .collect()
    }

    fn to_contract_data(&self) -> Result<Vec<ParsedContractResponse>, NyxdError> {
        let mut response = Vec::new();

        for (message_index, msg) in self.msg_responses.iter().enumerate() {
            // unfortunately `Name` trait has not been derived for `MsgExecuteContractResponse`,
            // so we have to make an explicit string comparison instead
            if msg.type_url == "/cosmwasm.wasm.v1.MsgExecuteContractResponse" {
                response.push(ParsedContractResponse {
                    message_index,
                    response: msg.to_contract_response_data()?,
                })
            }
        }

        Ok(response)
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

#[cfg(test)]
mod tests {
    use super::*;

    // cross-checked against a live proof: the mainnet mixnet contract's `admin` item
    // was proved under exactly `0x03 || addr(32) || "admin"`.
    #[test]
    fn contract_storage_key_matches_wasmd_layout() {
        let contract: AccountId = "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr"
            .parse()
            .unwrap();

        let key = contract_storage_key(&contract, b"admin");

        let expected = vec![
            3, 244, 7, 33, 76, 223, 43, 88, 38, 216, 46, 11, 149, 84, 35, 90, 59, 177, 232, 179,
            191, 57, 251, 173, 211, 178, 70, 187, 59, 57, 130, 43, 151, 97, 100, 109, 105, 110,
        ];
        assert_eq!(key, expected);
        assert_eq!(key[0], 0x03);
        assert_eq!(key.len(), 1 + 32 + b"admin".len());
    }
}
