// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_geolocation_contract_common::{
    AgentPermissions, EntryKey, ExecuteMsg as GeolocationExecuteMsg, LocationPayload, Measurement,
    NymNodeLocation, Subject,
};
use nym_mixnet_contract_common::NodeId;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GeolocationSigningClient {
    async fn execute_geolocation_contract(
        &self,
        fee: Option<Fee>,
        msg: GeolocationExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    /// Submit a batch of measurements. The sender must be whitelisted with `can_measure`, and
    /// is taken from the signer rather than the message, so an agent cannot write under
    /// another agent's key.
    ///
    /// All or nothing: one rejected entry fails the whole transaction.
    async fn submit_measurements(
        &self,
        measurements: Vec<Measurement>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::SubmitMeasurements { measurements };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::SubmitMeasurements".into(),
            vec![],
        )
        .await
    }

    /// Relay a batch of node-signed self-declarations verbatim. The sender must be whitelisted
    /// with `can_relay_self_declared`.
    ///
    /// Keep these out of a measurement batch: a relay carries data the agent did not produce
    /// and whose signature it cannot fully pre-validate, so one bad artifact must not be able to
    /// fail an agent's whole measurement sweep. A batch naming the same node twice is rejected.
    async fn relay_self_declarations(
        &self,
        declarations: Vec<NymNodeLocation>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::RelaySelfDeclarations { declarations };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::RelaySelfDeclarations".into(),
            vec![],
        )
        .await
    }

    /// Create or replace an override entry. Admin only.
    async fn set_location_override(
        &self,
        subject: Subject,
        payload: LocationPayload,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::SetOverride { subject, payload };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::SetOverride".into(),
            vec![],
        )
        .await
    }

    /// Delete an override entry, leaving every other source for that subject untouched. Admin
    /// only. Removing an absent override is a no-op rather than an error.
    async fn remove_location_override(
        &self,
        subject: Subject,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::RemoveOverride { subject };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::RemoveOverride".into(),
            vec![],
        )
        .await
    }

    /// Add an agent to the whitelist, or change an existing agent's permissions. Admin only.
    async fn set_whitelisted_agent(
        &self,
        agent: String,
        permissions: AgentPermissions,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::SetWhitelistedAgent { agent, permissions };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::SetWhitelistedAgent".into(),
            vec![],
        )
        .await
    }

    /// Remove an agent from the whitelist. Admin only.
    ///
    /// Non-destructive: the agent's entries stay in storage and in the digest, and a conforming
    /// reader stops honouring them immediately because authorisation is evaluated against the
    /// current whitelist at read time. [`Self::remove_geolocation_entries`] cleans up
    /// afterwards, as hygiene rather than as the security control.
    async fn remove_whitelisted_agent(
        &self,
        agent: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::RemoveWhitelistedAgent { agent };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::RemoveWhitelistedAgent".into(),
            vec![],
        )
        .await
    }

    /// Delete the named entries. Admin only, and bounded by the contract's batch size.
    ///
    /// Keys are named explicitly rather than scoped to an agent, so page
    /// [`GeolocationQueryClient::get_all_geolocation_records_paged`][q] off-chain and decide what
    /// should go before calling this.
    ///
    /// [q]: crate::nyxd::contract_traits::GeolocationQueryClient::get_all_geolocation_records_paged
    async fn remove_geolocation_entries(
        &self,
        keys: Vec<EntryKey>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::RemoveEntries { keys };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::RemoveEntries".into(),
            vec![],
        )
        .await
    }

    /// Transfer the admin role. Admin only.
    async fn update_admin(
        &self,
        admin: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::UpdateAdmin { admin };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::UpdateAdmin".into(),
            vec![],
        )
        .await
    }

    /// Change the contract's tunables. Admin only; omitted fields keep their current value, and
    /// the result is validated as a whole, so a partial update cannot arrive field by field at a
    /// configuration instantiation would have refused.
    async fn update_geolocation_config(
        &self,
        max_skew_secs: Option<u64>,
        max_batch_size: Option<u32>,
        max_payload_size: Option<u32>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::UpdateConfig {
            max_skew_secs,
            max_batch_size,
            max_payload_size,
        };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::UpdateConfig".into(),
            vec![],
        )
        .await
    }

    /// The mixnet contract's unbond callback.
    ///
    /// Present for completeness rather than for use: the contract only accepts this from the
    /// mixnet contract address it was instantiated with, and rejects every other sender. In
    /// production it arrives as a sub-message of `UnbondNymNode`, not as a transaction anyone
    /// signs.
    async fn on_nym_node_unbond(
        &self,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = GeolocationExecuteMsg::OnNymNodeUnbond { node_id };
        self.execute_geolocation_contract(
            fee,
            msg,
            "GeolocationExecuteMsg::OnNymNodeUnbond".into(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> GeolocationSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_geolocation_contract(
        &self,
        fee: Option<Fee>,
        msg: GeolocationExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let contract_address = &self
            .geolocation_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("geolocation contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()[0];
        self.execute(signer_address, contract_address, &msg, fee, memo, funds)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: GeolocationSigningClient + Send + Sync>(
        client: C,
        msg: GeolocationExecuteMsg,
    ) {
        match msg {
            GeolocationExecuteMsg::SubmitMeasurements { measurements } => {
                client.submit_measurements(measurements, None).ignore()
            }
            GeolocationExecuteMsg::RelaySelfDeclarations { declarations } => {
                client.relay_self_declarations(declarations, None).ignore()
            }
            GeolocationExecuteMsg::SetOverride { subject, payload } => client
                .set_location_override(subject, payload, None)
                .ignore(),
            GeolocationExecuteMsg::RemoveOverride { subject } => {
                client.remove_location_override(subject, None).ignore()
            }
            GeolocationExecuteMsg::SetWhitelistedAgent { agent, permissions } => client
                .set_whitelisted_agent(agent, permissions, None)
                .ignore(),
            GeolocationExecuteMsg::RemoveWhitelistedAgent { agent } => {
                client.remove_whitelisted_agent(agent, None).ignore()
            }
            GeolocationExecuteMsg::RemoveEntries { keys } => {
                client.remove_geolocation_entries(keys, None).ignore()
            }
            GeolocationExecuteMsg::UpdateAdmin { admin } => {
                client.update_admin(admin, None).ignore()
            }
            GeolocationExecuteMsg::UpdateConfig {
                max_skew_secs,
                max_batch_size,
                max_payload_size,
            } => client
                .update_geolocation_config(max_skew_secs, max_batch_size, max_payload_size, None)
                .ignore(),
            GeolocationExecuteMsg::OnNymNodeUnbond { node_id } => {
                client.on_nym_node_unbond(node_id, None).ignore()
            }
        };
    }
}
