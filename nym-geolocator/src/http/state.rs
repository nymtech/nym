// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::burst::BurstLimiter;
use crate::http::error::RequestError;
use crate::http::replay::ReplayGuard;
use crate::ip_info_lookup::{IpInfoLookup, LookupError};
use crate::node_scraper::NodeScraper;
use crate::nyx::client::NyxClient;
use crate::nyx::location_pusher::LocationPusher;
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::payload::Location;
use nym_geolocation_contract_common::{ContractConfig, Method, NymNodeLocation, Source, Subject};
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::contract_traits::{
    GeolocationQueryClient, GeolocationSigningClient,
};
use nym_validator_client::nyxd::nym_mixnet_contract_common::Addr;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) client: NyxClient,
    pub(crate) contract_config: ContractConfig,
    pub(crate) scraper: NodeScraper,
    pub(crate) location_pusher: LocationPusher,
    pub(crate) ip_info_lookup: IpInfoLookup,
    pub(crate) replay_guard: ReplayGuard,
    pub(crate) burst_limiter: BurstLimiter,
}

impl AppState {
    /// The location this agent currently has stored on chain for a node, if any.
    ///
    /// Read from the contract rather than from the local [`crate::nyx::state::OnChainNodes`]
    /// cache, which is what decides whether a node-requested measurement changed anything. The
    /// cache is maintained optimistically from what was submitted, so a submission that the chain
    /// later rejected would leave it claiming a value the contract never took, and a node would be
    /// charged against its allowance for a difference that does not exist.
    pub(crate) async fn stored_measurement(
        &self,
        node_id: NodeId,
    ) -> Result<Option<Location>, RequestError> {
        let agent = self.client.address().await;
        let source = Source::Measured {
            method: Method::IpInfo,
            agent: Addr::unchecked(agent.to_string()),
        };

        let response = self
            .client
            .get_location_entry(Subject::new_nym_node(node_id), source)
            .await
            .map_err(|err| {
                warn!("failed to read the stored location of node {node_id}: {err}");
                RequestError::upstream_failure(format!(
                    "could not read the stored location of node {node_id}"
                ))
            })?;

        let Some(entry) = response.entry else {
            return Ok(None);
        };

        // an entry this agent wrote that it can no longer decode is not something to charge the
        // node for, so it is reported as absent and the request counts as a change
        Ok(entry.payload.try_decode_v1().ok())
    }

    /// Reject the request if this agent is not whitelisted to relay self-declarations.
    ///
    /// Read per request rather than cached at startup, unlike the contract tunables: losing the
    /// permission is an ordinary admin action rather than a redeploy, and an agent that quietly
    /// went on accepting relays it could no longer submit would be worse than a query.
    pub(crate) async fn ensure_can_relay(&self) -> Result<(), RequestError> {
        let agent = self.client.address().await;

        let whitelist = self
            .client
            .get_geolocation_whitelist()
            .await
            .map_err(|err| {
                warn!("failed to read the agent whitelist: {err}");
                RequestError::upstream_failure("could not read the agent whitelist")
            })?;

        let permitted = whitelist
            .agents
            .iter()
            .find(|entry| entry.agent.as_str() == agent.as_ref())
            .is_some_and(|entry| entry.permissions.can_relay_self_declared);

        if !permitted {
            return Err(RequestError::forbidden(
                "this agent is not authorised to relay self-declarations",
            ));
        }

        Ok(())
    }

    /// The `declared_at` of the self-declaration currently stored for a node, if it has one.
    async fn stored_declaration_time(&self, node_id: NodeId) -> Result<Option<u64>, RequestError> {
        let response = self
            .client
            .get_location_entry(Subject::new_nym_node(node_id), Source::SelfDeclared)
            .await
            .map_err(|err| {
                warn!("failed to read the stored declaration of node {node_id}: {err}");
                RequestError::upstream_failure(format!(
                    "could not read the stored declaration of node {node_id}"
                ))
            })?;

        Ok(response
            .entry
            .and_then(|entry| entry.attestation)
            .map(|attestation| attestation.declared_at))
    }

    /// Reject an artifact the contract would reject anyway.
    ///
    /// Purely an optimisation: the contract performs all of this itself, and remains the control
    /// that matters. Doing it here turns a bad artifact into a rejected request rather than a
    /// failed transaction, and gives the node an answer that says which rule it broke.
    pub(crate) async fn prevalidate_declaration(
        &self,
        declaration: &NymNodeLocation,
        identity_key: &ed25519::PublicKey,
    ) -> Result<(), RequestError> {
        let node_id = declaration.node_id;

        // verified against the artifact's own bytes, exactly as the contract will: the payload
        // builds the signed message from itself, so there is nothing to disagree about
        let signature = ed25519::Signature::from_bytes(declaration.signature.as_slice())
            .map_err(|err| RequestError::unauthorised(format!("malformed signature: {err}")))?;
        identity_key
            .verify(declaration.signing_payload(), &signature)
            .map_err(|_| {
                RequestError::unauthorised(format!(
                    "the declaration was not signed by the identity key of node {node_id}"
                ))
            })?;

        declaration
            .payload
            .ensure_within_size_limit(self.contract_config.max_payload_size)
            .map_err(|err| RequestError::bad_request(err.to_string()))?;

        let now = OffsetDateTime::now_utc().unix_timestamp() as u64;
        if declaration.declared_at > now + self.contract_config.max_skew_secs {
            // a node whose clock runs fast would otherwise be rejected on chain with an error
            // that reads as the geolocator being broken
            return Err(RequestError::bad_request(format!(
                "declared_at is further ahead than the permitted {} seconds",
                self.contract_config.max_skew_secs
            )));
        }

        if let Some(stored) = self.stored_declaration_time(node_id).await? {
            if declaration.declared_at <= stored {
                return Err(RequestError::conflict(format!(
                    "a declaration at least as recent is already stored for node {node_id}"
                )));
            }
        }

        Ok(())
    }

    /// Relay a verified artifact to the contract, in a transaction of its own.
    ///
    /// Never batched with measurements: its acceptance turns on contract state this service does
    /// not control, and a sweep's worth of measurements must not be lost to one bad artifact.
    pub(crate) async fn relay_declaration(
        &self,
        declaration: NymNodeLocation,
    ) -> Result<(), RequestError> {
        let node_id = declaration.node_id;
        let declared_at = declaration.declared_at;

        let Err(err) = self
            .client
            .relay_self_declarations(vec![declaration], None)
            .await
        else {
            return Ok(());
        };

        // another agent relaying the same artifact first is the expected outcome of a node
        // announcing to several of them, not a fault. established by re-reading rather than by
        // matching on the rejection text, which is not ours to depend on
        if let Ok(Some(stored)) = self.stored_declaration_time(node_id).await {
            if stored >= declared_at {
                return Err(RequestError::conflict(format!(
                    "a declaration at least as recent is already stored for node {node_id}"
                )));
            }
        }

        warn!("failed to relay the declaration of node {node_id}: {err}");
        Err(RequestError::upstream_failure(format!(
            "could not relay the declaration of node {node_id}"
        )))
    }

    /// Measure a single node now and submit the result, returning what was submitted.
    ///
    /// Shared by both authentication modes of the re-test endpoint, which differ only in who is
    /// allowed to ask and how often, never in what gets done. In particular it never touches the
    /// [`BurstLimiter`]: that is the node-signed handler's business alone, so neither the regular
    /// sweep nor a bearer-token request can consume a node's allowance.
    pub(crate) async fn measure_and_submit(
        &self,
        node_id: NodeId,
    ) -> Result<Location, RequestError> {
        // deliberately re-discovered rather than read out of the baseline: a re-test is a request
        // to measure the node as it is now, and the baseline may be a full refresh cycle old.
        // this also records what it finds, so the next sweep does not see its own stale baseline
        // as a change and measure the node a second time
        let Some(discovered) = self.scraper.refresh_node(node_id).await else {
            return Err(RequestError::upstream_failure(format!(
                "could not discover the addresses of node {node_id}"
            )));
        };
        let addresses = discovered.addresses;

        let location = match self
            .ip_info_lookup
            .lookup_node_location(addresses.clone())
            .await
        {
            Ok(Some(location)) => location,
            // an unlocatable node must not be written as an empty location, exactly as in the
            // regular sweep: the previous entry is left alone and the caller is told it failed
            Ok(None) => {
                return Err(RequestError::upstream_failure(format!(
                    "node {node_id} announced no addresses to geolocate"
                )));
            }
            // the lookup provider is shared with the regular sweep and serves one caller at a
            // time, so a request arriving mid-sweep is turned away rather than held
            Err(LookupError::Busy) => {
                return Err(RequestError::busy(
                    "the lookup provider is busy - please try again",
                ));
            }
            Err(LookupError::Failed(err)) => {
                warn!("failed to look up the location of node {node_id}: {err}");
                return Err(RequestError::upstream_failure(format!(
                    "could not determine the location of node {node_id}"
                )));
            }
        };

        self.location_pusher
            .push_updates(vec![(node_id, location.clone())])
            .await
            .map_err(|err| {
                warn!("failed to submit the location of node {node_id}: {err}");
                RequestError::upstream_failure(format!(
                    "could not submit the location of node {node_id}"
                ))
            })?;

        // the sweep tracks what was measured rather than what was discovered, so a re-test that
        // reached the chain has to say so - otherwise the next pass measures this node again for
        // a change it has already accounted for
        self.scraper.mark_measured(vec![(node_id, addresses)]).await;

        Ok(location)
    }
}
