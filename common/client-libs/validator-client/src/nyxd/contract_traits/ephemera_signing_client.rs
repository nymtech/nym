// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_ephemera_common::msg::ExecuteMsg as EphemeraExecuteMsg;
use nym_ephemera_common::types::JsonPeerInfo;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait EphemeraSigningClient {
    async fn execute_ephemera_contract(
        &self,
        fee: Option<Fee>,
        msg: EphemeraExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn register_as_peer(
        &self,
        peer_info: JsonPeerInfo,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = EphemeraExecuteMsg::RegisterPeer { peer_info };

        self.execute_ephemera_contract(fee, req, "registering as peer".to_string(), vec![])
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> EphemeraSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_ephemera_contract(
        &self,
        fee: Option<Fee>,
        msg: EphemeraExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let ephemera_contract_address = self
            .ephemera_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("ephemera contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let signer_address = &self.signer_addresses()?[0];

        self.execute(
            signer_address,
            ephemera_contract_address,
            &msg,
            fee,
            memo,
            funds,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: EphemeraSigningClient + Send + Sync>(
        client: C,
        msg: EphemeraExecuteMsg,
    ) {
        match msg {
            EphemeraExecuteMsg::RegisterPeer { peer_info } => {
                client.register_as_peer(peer_info, None).ignore()
            }
        };
    }
}
