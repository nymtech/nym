// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimNymClient`] — simulated client that produces sphinx-in-LP packets
//! consumed by the [`SimNymNode`](crate::node::nymnode::SimNymNode).
//!
//! Both directions are the real client's: [`LpOutboundPipeline`] wraps what this client sends and
//! [`LpInboundPipeline`] unwraps what arrives, the same two a `nym-client` runs. What the simulation
//! supplies is only what a client would otherwise learn from a directory and a control plane - a
//! topology, a session per node, and who to address.

use std::{sync::Arc, time::Instant};

use getrandom04::SysRng;
use nym_client_core::client::lp::data::handler::pipeline::inbound::LpInboundPipeline;
use nym_client_core::client::lp::data::handler::pipeline::outbound::{
    LpOutboundOptions, LpOutboundPipeline,
};
use nym_client_core::client::lp::data::shared::{LpGatewaySessions, SharedLpDataState};
use nym_client_core::client::topology_control::TopologyAccessor;
use nym_client_core::config::DebugConfig;
use nym_crypto::asymmetric::x25519;
use nym_lp::peer::LpLocalPeer;
use nym_lp_data::{
    AddressedTimedData,
    clients::traits::{ClientUnwrappingPipeline, ClientWrappingPipeline},
    packet::EncryptedLpPacket,
};
use nym_sphinx_addressing::ClientAddress;
use rand::{Rng, rngs::OsRng};
use rand010::SeedableRng;

use crate::{
    client::{BaseClient, ClientId, ProcessingClient},
    peers::random_peer_mlkem_only,
    topology::{TopologyClient, directory::Directory},
};

/// A simulated client that produces sphinx-in-LP packets.
///
/// `Ts` is fixed to [`Instant`] because the real pipelines only work on wall-clock time.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the wrapping/unwrapping pipelines.
pub type SimNymClient<R> = BaseClient<SimNymProcesssingClient<R>, EncryptedLpPacket>;

impl<R: Rng + Send> SimNymClient<R> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(
        topology_client: TopologyClient,
        directory: Arc<Directory>,
        rng: R,
    ) -> anyhow::Result<(Self, SimNymClientLpIdentity)> {
        // LP keys are generated per run, as they are for nodes: the simulation carries no identity
        // across runs and an ML-KEM768 keypair would be kilobytes of JSON per client.
        let mut key_rng = rand010::rngs::StdRng::try_from_rng(&mut SysRng)?;
        let local_peer = random_peer_mlkem_only(&mut key_rng);

        // the driver fills this in, one session per node - which is what a real client would have
        // got from registering, except that it registers with every node here
        let sessions = LpGatewaySessions::default();
        let shared_state = Arc::new(SharedLpDataState::new(sessions.clone()));

        let identity = SimNymClientLpIdentity {
            local_peer,
            sessions,
            client_address: directory
                .client(topology_client.client_id)
                .map(|client| client.client_address)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "client {} is not in the directory",
                        topology_client.client_id
                    )
                })?,
        };

        // what a client would refresh from a nym-api, standing still for the whole run
        let topology_accessor = TopologyAccessor::new(false);
        topology_accessor.manually_change_topology(directory.as_nym_topology());

        // the sphinx key this client is the final hop with, as a keypair the pipeline can peel with
        let sphinx_public = x25519::PublicKey::from(&topology_client.sphinx_private_key);
        let sphinx_keys = x25519::KeyPair::from_bytes(
            &topology_client.sphinx_private_key.to_bytes(),
            &sphinx_public.to_bytes(),
        )?;

        let processing_client = SimNymProcesssingClient {
            directory: directory.clone(),
            rng,
            wrapper: LpOutboundPipeline::new(
                OsRng,
                DebugConfig::default(),
                topology_accessor,
                shared_state.clone(),
            ),
            unwrapper: LpInboundPipeline::new(shared_state, Arc::new(sphinx_keys)),
        };

        let client = BaseClient::with_pipeline(
            topology_client.client_id,
            topology_client.mixnet_address,
            topology_client.app_address,
            processing_client,
        )?;

        Ok((client, identity))
    }
}

/// What a client has to expose for the driver to pair it with the nodes.
///
/// A client's entry is drawn per packet, so it handshakes with every node rather than one gateway.
/// The node side of each session is filed under [`Self::client_address`] - that, not an IP, is how a
/// node addresses a client.
pub struct SimNymClientLpIdentity {
    pub local_peer: LpLocalPeer,
    pub sessions: LpGatewaySessions,
    pub client_address: ClientAddress,
}

/// Drives the real client pipelines from the simulation's own clock and directory.
pub struct SimNymProcesssingClient<R: Rng> {
    /// Where the entry node for each packet is drawn from, and who its recipients are.
    directory: Arc<Directory>,

    /// Draws that entry node. The pipelines carry their own randomness.
    rng: R,

    wrapper: LpOutboundPipeline<OsRng>,
    unwrapper: LpInboundPipeline,
}

impl<R: Rng + Send> ProcessingClient<EncryptedLpPacket> for SimNymProcesssingClient<R> {
    fn process(
        &mut self,
        input: Vec<u8>,
        dst: ClientId,
        timestamp: Instant,
    ) -> Vec<AddressedTimedData<EncryptedLpPacket>> {
        if input.is_empty() {
            return Vec::new();
        }

        let Some(recipient) = self.directory.recipient_of(dst) else {
            tracing::error!("Destination {dst} does not exist in the topology");
            return Vec::new();
        };

        // any gateway will do, and a gateway is never a mix layer, so whichever is drawn is never
        // the first hop the pipeline then asks it to forward to
        let Some(entry) = self.directory.random_gateway(&mut self.rng) else {
            tracing::error!("the topology has no gateway to send through");
            return Vec::new();
        };

        self.wrapper
            .process(
                Some((input, LpOutboundOptions { recipient }, entry.addr)),
                timestamp,
            )
            .inspect_err(|e| tracing::error!("Failed to wrap a packet for {}: {e}", entry.addr))
            .unwrap_or_default()
    }

    fn unwrap(
        &mut self,
        input: EncryptedLpPacket,
        timestamp: Instant,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.unwrapper.unwrap(input, timestamp)?)
    }
}
