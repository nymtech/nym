// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::adpu_answer::answer_bytes;
use crate::error::LedgerError;
use ledger_transport::APDUAnswer;

pub struct VersionResponse {
    pub test_mode: bool,
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub device_locked: bool,
}

impl TryFrom<APDUAnswer<Vec<u8>>> for VersionResponse {
    type Error = crate::error::LedgerError;

    fn try_from(answer: APDUAnswer<Vec<u8>>) -> Result<Self, Self::Error> {
        let bytes = answer_bytes(&answer)?;
        let received = bytes.len();
        let err =
            |expected| -> LedgerError { Self::Error::InvalidAnswerLength { expected, received } };

        let test_mode = *bytes.get(0).ok_or_else(|| err(0))? != 0;
        let major = *bytes.get(1).ok_or_else(|| err(1))?;
        let minor = *bytes.get(2).ok_or_else(|| err(2))?;
        let patch = *bytes.get(3).ok_or_else(|| err(3))?;
        let device_locked = *bytes.get(4).ok_or_else(|| err(4))? != 0;

        Ok(VersionResponse {
            test_mode,
            major,
            minor,
            patch,
            device_locked,
        })
    }
}
