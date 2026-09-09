// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Running the simulator in a test.
//!
//! [`SimHarness`] builds a topology, wires every participant to one in-process network, and steps
//! the driver on a virtual clock - so a test asserts on what was delivered, in what order, over
//! which route, without a socket or a sleep anywhere.

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

use rand::{SeedableRng, rngs::StdRng};

use crate::{
    client::ClientId,
    driver::{MixSimDriver, NymNodeMixDriver},
    logging::ClientTraces,
    node::NodeId,
    sim::env::SeededEnv,
    topology::{NodeRole, Topology, TopologyClient, TopologyNode},
    transport::memory::{MemoryNetwork, NetworkTrace},
};

pub mod env;

/// Port numbers are decoration here - addresses are never bound - but keeping the ones the CLI uses
/// means a seeded topology reads like a generated one.
const NODE_PORT: u16 = 51264;
const CLIENT_MIX_PORT: u16 = 9000;
const CLIENT_APP_PORT: u16 = 9001;

/// Stands in for the `mix-client` binary as the source of an injected payload.
const INJECTOR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 255)), 1);

/// How much virtual time one [`SimHarness::step`] covers.
const TICK: Duration = Duration::from_millis(10);

/// Assemble a [`SimHarness`].
pub struct SimHarnessBuilder {
    seed: [u8; 32],
    gateways: u8,
    clients: u8,
}

impl Default for SimHarnessBuilder {
    fn default() -> Self {
        SimHarnessBuilder {
            seed: [0; 32],
            gateways: 2,
            clients: 2,
        }
    }
}

impl SimHarnessBuilder {
    /// Fix the run: same seed, same route, same order, same delivery.
    pub fn seed(mut self, seed: [u8; 32]) -> Self {
        self.seed = seed;
        self
    }

    /// How many gateways to spread the clients across.
    ///
    /// Two or more is what gives [`Directory::gateway_of`] a choice to make, so a test can show
    /// that a different seed picks a different one.
    ///
    /// [`Directory::gateway_of`]: crate::topology::directory::Directory::gateway_of
    pub fn gateways(mut self, gateways: u8) -> Self {
        self.gateways = gateways;
        self
    }

    pub fn clients(mut self, clients: u8) -> Self {
        self.clients = clients;
        self
    }

    /// Build the topology, hand every participant an in-memory endpoint, and run the LP handshakes.
    ///
    /// Async only because the handshakes are.
    pub async fn build(self) -> anyhow::Result<SimHarness> {
        let mut env = SeededEnv::new(self.seed);
        let network = env.network();

        let mut key_rng = StdRng::from_seed(self.seed);
        let topology = generate_topology(&mut key_rng, self.gateways, self.clients);

        let app_addresses = topology
            .clients
            .iter()
            .map(|client| (client.client_id, client.app_address))
            .collect();
        let client_ids = topology
            .clients
            .iter()
            .map(|client| client.client_id)
            .collect();

        let client_traces = env.client_traces();
        let driver = NymNodeMixDriver::new(topology, &mut env)
            .await?
            .into_inner();

        Ok(SimHarness {
            driver,
            network,
            client_traces,
            app_addresses,
            client_ids,
            clock: Instant::now(),
            ticks: 0,
        })
    }
}

/// One node per mix layer, `gateways` gateways, `clients` clients, each on its own IP.
///
/// A single node per layer is deliberate: `choose_mixing_node` then has one candidate per hop, so
/// the `HashSet` iteration order inside `nym-topology` cannot reach the route. Gateways are where
/// the run keeps a genuine choice to make.
fn generate_topology(rng: &mut StdRng, gateways: u8, clients: u8) -> Topology {
    let roles = [NodeRole::Layer1, NodeRole::Layer2, NodeRole::Layer3]
        .into_iter()
        .chain(std::iter::repeat_n(NodeRole::Gateway, gateways as usize));

    let nodes: Vec<_> = roles
        .enumerate()
        .map(|(index, role)| {
            let node_id = index as NodeId + 1;
            TopologyNode::new_with_rng(rng, node_id, 100, address(node_id, NODE_PORT), role)
        })
        .collect();

    let first_client = nodes.len() as ClientId + 1;
    let clients = (0..clients)
        .map(|index| {
            let client_id = first_client + index;
            TopologyClient::new_with_rng(
                rng,
                client_id,
                address(client_id, CLIENT_MIX_PORT),
                address(client_id, CLIENT_APP_PORT),
            )
        })
        .collect();

    Topology { nodes, clients }
}

/// Distinct per participant, because LP sessions between nodes are keyed by IP.
fn address(id: u8, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, id)), port)
}

/// A whole simulated network, stepped by hand.
pub struct SimHarness {
    driver: MixSimDriver,
    network: MemoryNetwork,
    client_traces: ClientTraces,
    app_addresses: BTreeMap<ClientId, SocketAddr>,
    client_ids: Vec<ClientId>,
    clock: Instant,
    ticks: usize,
}

impl SimHarness {
    pub fn builder() -> SimHarnessBuilder {
        SimHarnessBuilder::default()
    }

    /// Every client in the topology, in id order.
    pub fn clients(&self) -> &[ClientId] {
        &self.client_ids
    }

    /// Hand `payload` to `from` for delivery to `to`.
    ///
    /// Written to the sender's app endpoint in the `[dst, payload..]` framing `mix-client` uses, so
    /// this enters by the same door an interactive run does.
    pub fn send(&self, from: ClientId, to: ClientId, payload: &[u8]) -> anyhow::Result<()> {
        let app_address = self
            .app_addresses
            .get(&from)
            .ok_or_else(|| anyhow::anyhow!("client {from} is not in the topology"))?;

        let mut framed = Vec::with_capacity(payload.len() + 1);
        framed.push(to);
        framed.extend_from_slice(payload);

        // Not recording data we're telling a client to send
        self.network.send_to(INJECTOR, *app_address, framed, false);
        Ok(())
    }

    /// Advance the simulation by one tick.
    pub fn step(&mut self) {
        self.driver.tick(self.clock, false);
        self.clock += TICK;
        self.ticks += 1;
    }

    /// Step until `done` holds, or until `max_ticks` have passed.
    ///
    /// Returns whether `done` held, so a test can assert on it rather than on a tick count.
    pub fn step_until(&mut self, max_ticks: usize, done: impl Fn(&ClientTraces) -> bool) -> bool {
        for _ in 0..max_ticks {
            if done(&self.client_traces) {
                return true;
            }
            self.step();
        }
        done(&self.client_traces)
    }

    /// What `client` has received, in arrival order.
    pub fn delivered_to(&self, client: ClientId) -> Vec<Vec<u8>> {
        self.client_traces.for_client(client)
    }

    pub fn client_traces(&self) -> &ClientTraces {
        &self.client_traces
    }

    /// Every send so far, in order - the route the run actually took.
    pub fn network_traces(&self) -> Vec<NetworkTrace> {
        self.network.network_traces()
    }

    /// The route as `(src, dst)` pairs, which is what stays fixed across runs of one seed.
    pub fn route(&self) -> Vec<(SocketAddr, SocketAddr)> {
        self.network
            .network_traces()
            .into_iter()
            .map(|trace| (trace.src, trace.dst))
            .collect()
    }

    /// Datagrams sent to an address nobody listens on. Should be zero.
    pub fn unroutable(&self) -> usize {
        self.network.unroutable()
    }

    /// How many ticks have been taken.
    pub fn ticks(&self) -> usize {
        self.ticks
    }
}
