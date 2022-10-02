// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{LedgerError, Result};
use bip32::DerivationPath;
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

pub(crate) fn path_bytes(path: DerivationPath) -> Result<[[u8; 4]; 5]> {
    let received = path.len();
    let components: Vec<[u8; 4]> = path.into_iter().map(|c| c.0.to_le_bytes()).collect();
    if components.len() != 5 {
        Err(LedgerError::InvalidDerivationPath {
            expected: 5,
            received,
        })
    } else {
        Ok([
            components[0],
            components[1],
            components[2],
            components[3],
            components[4],
        ])
    }
}
