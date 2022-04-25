// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use cosmrs::proto::cosmos::base::query::v1beta1::{PageRequest, PageResponse};
use cosmrs::proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmrs::rpc::endpoint::broadcast;
use cosmrs::Coin;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

pub(crate) trait CheckResponse: Sized {
    fn check_response(self) -> Result<Self, NymdError>;
}

impl CheckResponse for broadcast::tx_commit::Response {
    fn check_response(self) -> Result<Self, NymdError> {
        if self.check_tx.code.is_err() {
            return Err(NymdError::BroadcastTxErrorCheckTx {
                hash: self.hash,
                height: self.height,
                code: self.check_tx.code.value(),
                raw_log: self.check_tx.log.value().to_owned(),
            });
        }

        if self.deliver_tx.code.is_err() {
            return Err(NymdError::BroadcastTxErrorDeliverTx {
                hash: self.hash,
                height: self.height,
                code: self.deliver_tx.code.value(),
                raw_log: self.deliver_tx.log.value().to_owned(),
            });
        }

        Ok(self)
    }
}

pub(crate) fn compress_wasm_code(code: &[u8]) -> Result<Vec<u8>, NymdError> {
    // using compression level 9, same as cosmjs, that optimises for size
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(code)
        .map_err(NymdError::WasmCompressionError)?;
    encoder.finish().map_err(NymdError::WasmCompressionError)
}

pub(crate) fn create_pagination(key: Vec<u8>) -> PageRequest {
    PageRequest {
        key,
        offset: 0,
        limit: 0,
        count_total: false,
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

pub(crate) fn parse_proto_coin_vec(value: Vec<ProtoCoin>) -> Result<Vec<Coin>, NymdError> {
    value
        .into_iter()
        .map(|proto_coin| {
            Coin::try_from(&proto_coin).map_err(|_| NymdError::MalformedCoin {
                coin_representation: format!("{:?}", proto_coin),
            })
        })
        .collect()
}
