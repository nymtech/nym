// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The whole network as one in-process switchboard.
//!
//! Nothing is bound, so a [`SocketAddr`] here is a pure name - but every participant still needs a
//! distinct one, because LP sessions between nodes are keyed by IP.
//!
//! Every datagram passes through [`MemoryNetwork`], which is what makes a run's route assertable:
//! [`MemoryNetwork::network_traces`] is the sequence of sends, recorded without a line of
//! instrumentation anywhere in the node or client.

use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{self, TryRecvError},
    },
};

use crate::transport::SimEndpoint;

/// A single send, in the order it happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkTrace {
    pub src: SocketAddr,
    pub dst: SocketAddr,
    /// Serialised length. Recorded for diagnostics; the route is `(src, dst)`.
    pub len: usize,
}

#[derive(Default)]
struct NetworkInner {
    inboxes: HashMap<SocketAddr, mpsc::Sender<(SocketAddr, Vec<u8>)>>,

    // Metrics for tests
    network_traces: Vec<NetworkTrace>,
    unroutable: usize,
}

impl NetworkInner {
    fn deliver(&mut self, src: SocketAddr, dst: SocketAddr, bytes: Vec<u8>, record_trace: bool) {
        let len = bytes.len();
        let delivered = self
            .inboxes
            .get(&dst)
            .is_some_and(|inbox| inbox.send((src, bytes)).is_ok());
        if record_trace {
            self.network_traces.push(NetworkTrace { src, dst, len });
        }
        if !delivered {
            self.unroutable += 1;
        }
    }
}

/// Every participant's inbox, and the log of what moved between them.
#[derive(Clone, Default)]
pub struct MemoryNetwork {
    inner: Arc<Mutex<NetworkInner>>,
}

impl MemoryNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recover rather than propagate: a poisoned lock means some other test thread panicked, and
    /// failing that test is more useful than failing every other one behind it.
    fn lock(&self) -> MutexGuard<'_, NetworkInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Register `address` and hand back the endpoint that receives on it.
    ///
    /// Registering an address twice replaces the inbox, which orphans the older endpoint - so build
    /// each participant once, as the driver does.
    pub fn endpoint(&self, address: SocketAddr) -> Box<dyn SimEndpoint> {
        let (tx, rx) = mpsc::channel();
        self.lock().inboxes.insert(address, tx);

        Box::new(MemoryEndpoint {
            address,
            inbox: rx,
            network: self.clone(),
        })
    }

    /// Deliver `bytes` to `dst` as if they came from `src`.
    ///
    /// Record the trace if asked for
    pub fn send_to(&self, src: SocketAddr, dst: SocketAddr, bytes: Vec<u8>, record_trace: bool) {
        let mut inner = self.lock();
        inner.deliver(src, dst, bytes, record_trace);
    }

    /// Every send so far, in order.
    pub fn network_traces(&self) -> Vec<NetworkTrace> {
        self.lock().network_traces.clone()
    }

    /// How many datagrams went to an address nobody is listening on.
    ///
    /// A real UDP send to an unbound localhost address succeeds and vanishes, so this counts what
    /// the networked mode would silently drop.
    pub fn unroutable(&self) -> usize {
        self.lock().unroutable
    }
}

/// One participant's inbox on a [`MemoryNetwork`].
pub struct MemoryEndpoint {
    address: SocketAddr,
    inbox: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    network: MemoryNetwork,
}

impl SimEndpoint for MemoryEndpoint {
    fn send_to(&self, dst: SocketAddr, bytes: &[u8]) -> io::Result<usize> {
        let len = bytes.len();

        self.network
            .send_to(self.address, dst, bytes.to_vec(), true);

        // Regardless of whether it reached its destination, mark as sent, UDP style
        Ok(len)
    }

    fn try_recv_from(&self, buf: &mut [u8]) -> io::Result<Option<(usize, SocketAddr)>> {
        let (src, bytes) = match self.inbox.try_recv() {
            Ok(datagram) => datagram,
            // the network holds a sender for every registered address, so `Disconnected` only
            // happens once it is gone - which reads the same as nothing waiting
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {
                return Ok(None);
            }
        };

        // truncate rather than fail, the way `recv_from` fills a short buffer
        let read = bytes.len().min(buf.len());
        let (target, _) = buf.split_at_mut(read);
        let (source, _) = bytes.split_at(read);
        target.copy_from_slice(source);

        Ok(Some((read, src)))
    }
}
