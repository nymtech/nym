// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod adpu_answer;
pub mod error;
pub mod version;

use crate::version::VersionResponse;
use error::Result;
use ledger_transport::APDUCommand;
use ledger_transport_hid::hidapi::HidApi;
use ledger_transport_hid::TransportNativeHID;

const CLA: u8 = 0x55;
const INS_GET_VERSION: u8 = 0x00;
const INS_SIGN_SECP256K1: u8 = 0x02;
const INS_GET_ADDR_SECP256K1: u8 = 0x04;

pub struct Ledger {
    transport: TransportNativeHID,
}

impl Ledger {
    pub fn new() -> Result<Self> {
        let api = HidApi::new()?;
        let transport = TransportNativeHID::new(&api)?;

        Ok(Ledger { transport })
    }

    pub fn get_version(&self) -> Result<VersionResponse> {
        let command = APDUCommand {
            cla: CLA,
            ins: INS_GET_VERSION,
            p1: 0,
            p2: 0,
            data: vec![],
        };
        let response = self.transport.exchange(&command)?;
        VersionResponse::try_from(response)
    }
}
