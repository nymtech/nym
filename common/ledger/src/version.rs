// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::LedgerError;
use crate::helpers::answer_bytes;
use ledger_transport::APDUAnswer;

/// Version and status data of the device.
pub struct VersionResponse {
    /// Activation status of test mode.
    pub test_mode: bool,
    /// Major part of Cosmos application version.
    pub major: u8,
    /// Minor part of Cosmos application version.
    pub minor: u8,
    /// Patch part of Cosmos application version.
    pub patch: u8,
    /// PIN locked status.
    pub device_locked: bool,
}

impl TryFrom<APDUAnswer<Vec<u8>>> for VersionResponse {
    type Error = LedgerError;

    fn try_from(answer: APDUAnswer<Vec<u8>>) -> Result<Self, Self::Error> {
        let bytes = answer_bytes(&answer)?;
        if bytes.len() != 5 {
            return Err(Self::Error::InvalidAnswerLength {
                expected: 5,
                received: bytes.len(),
            });
        }

        Ok(VersionResponse {
            test_mode: bytes[0] != 0,
            major: bytes[1],
            minor: bytes[2],
            patch: bytes[3],
            device_locked: bytes[4] != 0,
        })
    }
}
