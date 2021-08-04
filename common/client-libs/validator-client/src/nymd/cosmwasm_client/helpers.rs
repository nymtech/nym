// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use cosmos_sdk::rpc::endpoint::broadcast;

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

// fn check_broadcast_result(
//     &self,
//     response: broadcast::tx_commit::Response,
// ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
//     Ok(response)
// }
