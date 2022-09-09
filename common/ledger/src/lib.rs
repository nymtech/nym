// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod addr_secp265k1;
pub mod error;
pub(crate) mod helpers;
pub mod version;

use crate::addr_secp265k1::AddrSecp265k1Response;
use crate::helpers::path_bytes;
use crate::version::VersionResponse;
use bip32::DerivationPath;
use error::Result;
use ledger_transport::APDUCommand;
use ledger_transport_hid::hidapi::HidApi;
use ledger_transport_hid::TransportNativeHID;

const CLA: u8 = 0x55;
const INS_GET_VERSION: u8 = 0x00;
const _INS_SIGN_SECP256K1: u8 = 0x02;
const INS_GET_ADDR_SECP256K1: u8 = 0x04;

/// Manage hardware Ledger device with Cosmos specific operations, as described in the
/// specification: https://github.com/cosmos/ledger-cosmos/blob/main/docs/APDUSPEC.md
pub struct CosmosLedger {
    transport: TransportNativeHID,
}

impl CosmosLedger {
    /// Create the connection to the first Ledger device that we can find.
    pub fn new() -> Result<Self> {
        let api = HidApi::new()?;
        let transport = TransportNativeHID::new(&api)?;

        Ok(CosmosLedger { transport })
    }

    /// Get the version of the device.
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

    /// Get the SECP265K1 address of the device.
    pub fn get_addr_secp265k1(
        &self,
        path: DerivationPath,
        prefix: &str,
        display: bool,
    ) -> Result<AddrSecp265k1Response> {
        let display = if display { 1 } else { 0 };
        let components = path_bytes(path)?;
        let data: Vec<u8> = vec![
            [prefix.len() as u8].as_slice(),
            prefix.as_bytes(),
            components[0].as_slice(),
            components[1].as_slice(),
            components[2].as_slice(),
            components[3].as_slice(),
            components[4].as_slice(),
        ]
        .into_iter()
        .flatten()
        .copied()
        .collect();

        let command = APDUCommand {
            cla: CLA,
            ins: INS_GET_ADDR_SECP256K1,
            p1: display,
            p2: 0,
            data,
        };
        let response = self.transport.exchange(&command)?;
        AddrSecp265k1Response::try_from(response)
    }
}
