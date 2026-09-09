// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Everything that differs between a live run and a seeded one.
//!
//! The two modes share every line of node, client and driver code; they part company only here, so
//! a difference between them is a difference in one of these four methods.

use std::{net::SocketAddr, time::Duration};

use nym_client_core::config::DebugConfig;
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{
    logging::{ClientTraces, SimLogging, StdOutLogging},
    transport::{SimEndpoint, bind_udp, memory::MemoryNetwork},
};

/// What a participant needs from the world around it, however that world is provided.
pub trait SimEnv {
    /// The endpoint this participant sends and receives on.
    fn endpoint(&self, address: SocketAddr) -> anyhow::Result<Box<dyn SimEndpoint>>;

    /// A generator for one participant.
    ///
    /// Each call returns a fresh, independent stream, so a participant's randomness depends on how
    /// many were built before it but never on when it happens to run.
    fn rng(&mut self) -> StdRng;

    /// What the sphinx layer is built with.
    fn debug_config(&self) -> DebugConfig;

    /// What a client does with a message once it arrives.
    fn logging(&self) -> Box<dyn SimLogging>;
}

/// Give out a child generator without ever reusing the parent's stream.
fn fork(rng: &mut StdRng) -> StdRng {
    StdRng::from_seed(rng.r#gen())
}

/// Real sockets, real entropy: what the CLI runs.
pub struct LiveEnv;

impl SimEnv for LiveEnv {
    fn endpoint(&self, address: SocketAddr) -> anyhow::Result<Box<dyn SimEndpoint>> {
        bind_udp(address)
    }

    fn rng(&mut self) -> StdRng {
        StdRng::from_entropy()
    }

    fn debug_config(&self) -> DebugConfig {
        DebugConfig::default()
    }

    fn logging(&self) -> Box<dyn SimLogging> {
        Box::new(StdOutLogging)
    }
}

/// One in-process network and one seed, from which the whole run follows.
///
/// Reproducible in route, order and delivery. Not in bytes: LP keys, handshakes and sphinx headers
/// still draw from the OS, which none of those three depend on.
pub struct SeededEnv {
    network: MemoryNetwork,
    rng: StdRng,
    client_traces: ClientTraces,
    debug: DebugConfig,
}

impl SeededEnv {
    pub fn new(seed: [u8; 32]) -> Self {
        let mut debug = DebugConfig::default();

        // per-hop delays are drawn from `rand::thread_rng()` deep inside `sphinx-packet`, which
        // takes no rng to seed - zero delay short-circuits before that draw ever happens
        debug.traffic.average_packet_delay = Duration::ZERO;
        debug.acknowledgements.average_ack_delay = Duration::ZERO;

        SeededEnv {
            network: MemoryNetwork::new(),
            rng: StdRng::from_seed(seed),
            client_traces: ClientTraces::default(),
            debug,
        }
    }

    /// The switchboard every participant is wired to.
    pub fn network(&self) -> MemoryNetwork {
        self.network.clone()
    }

    /// The record every client writes into.
    pub fn client_traces(&self) -> ClientTraces {
        self.client_traces.clone()
    }
}

impl SimEnv for SeededEnv {
    fn endpoint(&self, address: SocketAddr) -> anyhow::Result<Box<dyn SimEndpoint>> {
        Ok(self.network.endpoint(address))
    }

    fn rng(&mut self) -> StdRng {
        fork(&mut self.rng)
    }

    fn debug_config(&self) -> DebugConfig {
        self.debug
    }

    fn logging(&self) -> Box<dyn SimLogging> {
        Box::new(self.client_traces.clone())
    }
}
