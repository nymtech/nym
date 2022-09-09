// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, LedgerError>;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("HID API - {0}")]
    HidAPI(#[from] ledger_transport_hid::hidapi::HidError),

    #[error("HID transport - {0}")]
    HidTransport(#[from] ledger_transport_hid::LedgerHIDError),

    #[error("Unknown error code - {err_code}")]
    UnknownErrorCode { err_code: u16 },

    #[error("APDU error - {reason}")]
    APDU { reason: String },

    #[error("Not enough bytes in answer. Expected at least {expected}, received {received}")]
    InvalidAnswerLength { expected: usize, received: usize },
}
