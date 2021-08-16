// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use cosmos_sdk::proto::cosmos::base::query::v1beta1::PageRequest;
use cosmos_sdk::rpc::endpoint::broadcast;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

pub(crate) trait CheckResponse: Sized {
    fn check_response(self) -> Result<Self, ValidatorClientError>;
}

impl CheckResponse for broadcast::tx_commit::Response {
    fn check_response(self) -> Result<Self, ValidatorClientError> {
        if self.check_tx.code.is_err() {
            return Err(ValidatorClientError::BroadcastTxErrorCheckTx {
                hash: self.hash,
                height: self.height,
                code: self.check_tx.code.value(),
                raw_log: self.check_tx.log.value().to_owned(),
            });
        }

        if self.deliver_tx.code.is_err() {
            return Err(ValidatorClientError::BroadcastTxErrorDeliverTx {
                hash: self.hash,
                height: self.height,
                code: self.deliver_tx.code.value(),
                raw_log: self.deliver_tx.log.value().to_owned(),
            });
        }

        Ok(self)
    }
}

pub(crate) fn compress_wasm_code(code: &[u8]) -> Result<Vec<u8>, ValidatorClientError> {
    // using compression level 9, same as cosmjs, that optimises for size
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(code)
        .map_err(ValidatorClientError::WasmCompressionError)?;
    encoder
        .finish()
        .map_err(ValidatorClientError::WasmCompressionError)
}

pub(crate) fn create_pagination(key: Vec<u8>) -> PageRequest {
    PageRequest {
        key,
        offset: 0,
        limit: 0,
        count_total: false,
    }
}
