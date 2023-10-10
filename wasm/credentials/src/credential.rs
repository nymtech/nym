// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmCredentialClientError;
use crate::opts::CredentialClientOpts;
use js_sys::Promise;
use nym_credential_storage::ephemeral_storage::EphemeralCredentialStorage;
use nym_credential_storage::models::CoconutCredential;
use nym_credential_storage::storage::Storage;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::{Config, CosmWasmCoin};
use nym_validator_client::DirectSigningReqwestRpcNyxdClient;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::console_log;
use wasm_utils::error::PromisableResult;

#[wasm_bindgen(js_name = acquireCredential)]
pub fn acquire_credential(mnemonic: String, amount: String, opts: CredentialClientOpts) -> Promise {
    future_to_promise(async move {
        acquire_credential_async(mnemonic, amount, opts)
            .await
            .map(|credential| {
                serde_wasm_bindgen::to_value(&credential).expect("this serialization can't fail")
            })
            .into_promise_result()
    })
}

async fn acquire_credential_async(
    mnemonic: String,
    amount: String,
    opts: CredentialClientOpts,
) -> Result<WasmCoconutCredential, WasmCredentialClientError> {
    // start by parsing mnemonic so that we could immediately move it into a Zeroizing wrapper
    let mnemonic = crate::helpers::parse_mnemonic(mnemonic)?;

    // why are we parsing into CosmWasmCoin and not "our" Coin?
    // simple. because it has the nicest 'FromStr' impl
    let amount: CosmWasmCoin =
        amount
            .parse()
            .map_err(|source| WasmCredentialClientError::MalformedCoin {
                source: Box::new(source),
            })?;

    if amount.amount.is_zero() {
        return Err(WasmCredentialClientError::ZeroCoinValue);
    }

    let network = match opts.network_details {
        Some(specified) => specified,
        None => {
            if let Some(true) = opts.use_sandbox {
                crate::helpers::minimal_coconut_sandbox()
            } else {
                NymNetworkDetails::new_mainnet()
            }
        }
    };

    let config = Config::try_from_nym_network_details(&network)?;

    // just get the first nyxd endpoint
    let nyxd_endpoint = network
        .endpoints
        .get(0)
        .ok_or(WasmCredentialClientError::NoNyxdEndpoints)?
        .try_nyxd_url()?;

    let client = DirectSigningReqwestRpcNyxdClient::connect_reqwest_with_mnemonic(
        config,
        nyxd_endpoint,
        mnemonic,
    );

    console_log!("starting the deposit...");
    let deposit_state = nym_bandwidth_controller::acquire::deposit(&client, amount).await?;
    console_log!("obtained voucher: {:#?}", deposit_state.voucher);

    // TODO: use proper persistent storage here. probably indexeddb like we have for our 'normal' wasm client
    let ephemeral_storage = EphemeralCredentialStorage::default();

    // store credential in the ephemeral storage...
    nym_bandwidth_controller::acquire::get_credential(&deposit_state, &client, &ephemeral_storage)
        .await?;

    // and immediately get it out!
    let cred = ephemeral_storage.get_next_coconut_credential().await?;

    Ok(cred.into())
}

#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
// we could have implemented all ts traits on normal CoconutCredential,
// but it felt very awkward to import those crates in there; plus the underlying type might change
pub struct WasmCoconutCredential {
    pub voucher_value: String,
    pub voucher_info: String,
    pub serial_number: String,
    pub binding_number: String,
    pub signature: String,
    pub epoch_id: String,
}

impl From<CoconutCredential> for WasmCoconutCredential {
    fn from(value: CoconutCredential) -> Self {
        WasmCoconutCredential {
            voucher_value: value.voucher_value,
            voucher_info: value.voucher_info,
            serial_number: value.serial_number,
            binding_number: value.binding_number,
            signature: value.signature,
            epoch_id: value.epoch_id,
        }
    }
}
