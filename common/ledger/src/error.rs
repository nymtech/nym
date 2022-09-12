// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, LedgerError>;

/// Ledger specific errors.
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

    #[error("Not enough components in derivation path. Expected {expected}, received {received}")]
    InvalidDerivationPath { expected: usize, received: usize },

    #[error("Bip32 - {0}")]
    Bip32(#[from] bip32::Error),

    #[error("Signature error - {0}")]
    Signature(#[from] k256::ecdsa::Error),

    #[error("No message found for signing transaction")]
    NoMessageFound,
}
