// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::LedgerError;
use crate::helpers::answer_bytes;
use k256::ecdsa::Signature;
use ledger_transport::APDUAnswer;

/// Version and status data of the device.
pub struct SignSecp265k1Response {
    /// DER encoded signature data
    pub signature: Signature,
}

impl TryFrom<APDUAnswer<Vec<u8>>> for SignSecp265k1Response {
    type Error = LedgerError;

    fn try_from(answer: APDUAnswer<Vec<u8>>) -> Result<Self, Self::Error> {
        let bytes = answer_bytes(&answer)?;

        Ok(SignSecp265k1Response {
            signature: Signature::from_der(bytes)?,
        })
    }
}
