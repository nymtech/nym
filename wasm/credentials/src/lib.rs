use std::str::FromStr;

use js_sys::Promise;
use thiserror::Error;
use wasm_bindgen::prelude::wasm_bindgen;

use nym_bandwidth_controller::acquire::deposit;
use nym_bandwidth_controller::acquire::state::State;
use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_coconut_interface::Signature;
use nym_credentials::coconut::utils::obtain_aggregate_signature;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nyxd::contract_traits::CoconutBandwidthSigningClient;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Coin, CosmWasmCoin};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use wasm_utils::wasm_error;

#[wasm_bindgen]
struct WasmCredentialClient {
    state: Option<State>,
    client: DirectSigningHttpRpcNyxdClient,
}

#[wasm_bindgen]
impl WasmCredentialClient {
    #[wasm_bindgen(constructor)]
    pub fn new(
        // network_details: NymNetworkDetails,
        mnemonic: String,
    ) -> Result<WasmCredentialClient, WasmCredentialClientError> {
        let network_details = NymNetworkDetails::new_mainnet();
        let client = WasmCredentialClient::create_client(network_details, mnemonic)?;
        Ok(WasmCredentialClient {
            state: None,
            client,
        })
    }

    #[wasm_bindgen]
    pub async fn deposit(
        &mut self,
        amount_with_denom: String,
    ) -> Result<(), WasmCredentialClientError> {
        match CosmWasmCoin::from_str(&amount_with_denom) {
            Ok(coin) => {
                let state = deposit(&self.client, Coin::from(coin)).await?;
                self.state = Some(state);
                Ok(())
            }
            Err(_e) => Err(WasmCredentialClientError::CoinParseError),
        }
    }

    #[wasm_bindgen]
    pub async fn get_credential(self) -> Result<Signature, WasmCredentialClientError> {
        let epoch_id = self.client.get_current_epoch().await?.epoch_id;
        let threshold = self
            .client
            .get_current_epoch_threshold()
            .await?
            .ok_or(BandwidthControllerError::NoThreshold)?;

        let coconut_api_clients = all_coconut_api_clients(&self.client, epoch_id).await?;

        match self.state {
            Some(state) => {
                let signature = obtain_aggregate_signature(
                    &self.state.params,
                    &self.state.voucher,
                    &coconut_api_clients,
                    threshold,
                )
                .await?;

                Ok(signature)
            }
            None => Err(WasmCredentialClientError::StateError),
        }
    }

    fn create_client(
        network_details: NymNetworkDetails,
        mnemonic: String,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NyxdError> {
        let nyxd_url = network_details.endpoints[0].nyxd_url.as_str();
        let config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            config,
            nyxd_url,
            mnemonic.parse()?,
        )?;

        Ok(client)
    }
}

#[derive(Debug, Error)]
pub enum WasmCredentialClientError {
    #[error(transparent)]
    BandwidthControllerError {
        #[from]
        source: BandwidthControllerError,
    },
    #[error("Coin parse error")]
    CoinParseError,
    #[error("State error")]
    StateError,
}

wasm_error!(WasmCredentialClientError);
