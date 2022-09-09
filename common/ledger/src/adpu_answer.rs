// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{LedgerError, Result};
use ledger_transport::{APDUAnswer, APDUErrorCode};

pub(crate) fn answer_bytes(answer: &APDUAnswer<Vec<u8>>) -> Result<&[u8]> {
    let error_code = answer
        .error_code()
        .map_err(|err_code| LedgerError::UnknownErrorCode { err_code })?;
    match error_code {
        APDUErrorCode::NoError => Ok(answer.data()),
        e => Err(LedgerError::APDU {
            reason: e.description(),
        }),
    }
}
