// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod addr_secp265k1;
pub mod error;
pub(crate) mod helpers;
mod sign_secp265k1;
pub mod version;

use crate::addr_secp265k1::AddrSecp265k1Response;
use crate::error::LedgerError;
use crate::helpers::path_bytes;
use crate::sign_secp265k1::SignSecp265k1Response;
use crate::version::VersionResponse;
use bip32::DerivationPath;
use error::Result;
use ledger_transport::APDUCommand;
use ledger_transport_hid::hidapi::HidApi;
use ledger_transport_hid::TransportNativeHID;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;

const CLA: u8 = 0x55;
const INS_GET_VERSION: u8 = 0x00;
const INS_SIGN_SECP256K1: u8 = 0x02;
const INS_GET_ADDR_SECP256K1: u8 = 0x04;

const PAYLOAD_TYPE_INIT: u8 = 0x00;
const PAYLOAD_TYPE_ADD: u8 = 0x01;
const PAYLOAD_TYPE_LAST: u8 = 0x02;
const CHUNK_SIZE: usize = 250;

/// Manage hardware Ledger device with Cosmos specific operations, as described in the
/// specification: https://github.com/cosmos/ledger-cosmos/blob/main/docs/APDUSPEC.md
#[derive(Clone)]
pub struct CosmosLedger {
    path: DerivationPath,
    prefix: String,
    transport: Arc<TransportNativeHID>,
}

impl Debug for CosmosLedger {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "()")
    }
}

impl CosmosLedger {
    /// Create the connection to the first Ledger device that we can find.
    pub fn new(path: DerivationPath, prefix: String) -> Result<Self> {
        let api = HidApi::new()?;
        let transport = Arc::new(TransportNativeHID::new(&api)?);

        Ok(CosmosLedger {
            path,
            prefix,
            transport,
        })
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
    pub fn get_addr_secp265k1(&self, display: bool) -> Result<AddrSecp265k1Response> {
        let display = if display { 1 } else { 0 };
        let components = path_bytes(self.path.clone())?;
        let data: Vec<u8> = vec![
            [self.prefix.len() as u8].as_slice(),
            self.prefix.as_bytes(),
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

    pub fn sign_secp265k1(&self, message: String) -> Result<SignSecp265k1Response> {
        let serialized_path: Vec<u8> = path_bytes(self.path.clone())?
            .into_iter()
            .flatten()
            .collect();
        let mut chunks = vec![serialized_path];
        if message.is_empty() {
            return Err(LedgerError::NoMessageFound);
        }
        for chunk in message.into_bytes().chunks(CHUNK_SIZE) {
            chunks.push(chunk.to_vec());
        }
        let length = chunks.len();
        for (idx, chunk) in chunks.into_iter().enumerate() {
            let payload_desc = if idx == 0 {
                PAYLOAD_TYPE_INIT
            } else if idx + 1 == length {
                PAYLOAD_TYPE_LAST
            } else {
                PAYLOAD_TYPE_ADD
            };
            let command = APDUCommand {
                cla: CLA,
                ins: INS_SIGN_SECP256K1,
                p1: payload_desc,
                p2: 0,
                data: chunk,
            };
            let sign_response = self.transport.exchange(&command)?;
            if payload_desc == PAYLOAD_TYPE_LAST {
                return SignSecp265k1Response::try_from(sign_response);
            }
        }
        // It should never reach this, as the message is not empty
        Err(LedgerError::NoMessageFound)
    }
}
