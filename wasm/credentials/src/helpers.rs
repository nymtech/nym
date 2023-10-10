// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmCredentialClientError;
use nym_network_defaults::{NymContracts, NymNetworkDetails, ValidatorDetails};
use zeroize::Zeroizing;

pub(crate) fn parse_mnemonic(raw: String) -> Result<bip39::Mnemonic, WasmCredentialClientError> {
    // make sure that whatever happens, the raw value gets zeroized
    let wrapped = Zeroizing::new(raw);
    Ok(bip39::Mnemonic::parse(&*wrapped)?)
}

pub(crate) fn minimal_coconut_sandbox() -> NymNetworkDetails {
    // we can piggyback on mainnet defaults for certain things,
    // since sandbox uses the same network name, denoms, etc.
    let default_mainnet = NymNetworkDetails::new_mainnet();

    NymNetworkDetails {
        network_name: default_mainnet.network_name,
        chain_details: default_mainnet.chain_details,
        endpoints: vec![ValidatorDetails::new(
            "https://sandbox-validator1.nymtech.net",
            None,
        )],
        contracts: NymContracts {
            coconut_bandwidth_contract_address: Some(
                "n16a32stm6kknhq5cc8rx77elr66pygf2hfszw7wvpq746x3uffylqkjar4l".into(),
            ),
            coconut_dkg_contract_address: Some(
                "n1ahg0erc2fs6xx3j5m8sfx3ryuzdjh6kf6qm9plsf865fltekyrfsesac6a".into(),
            ),
            // we don't need other contracts for getting credential
            ..Default::default()
        },
        explorer_api: None,
    }
}
