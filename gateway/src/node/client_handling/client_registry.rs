// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Where each registered client currently is.
//!
//! The counterpart to [`ActiveClientsStore`](super::active_clients::ActiveClientsStore): that one
//! holds the channels for clients connected over a socket this node owns, so an address never
//! comes up. Here the client is somewhere else on the network and has to be *addressed*, which
//! means tracking where it was last seen.

use dashmap::DashMap;
use nym_sphinx::addressing::ClientAddress;
use std::net::SocketAddr;
use std::sync::Arc;

/// Where each registered client was last seen.
///
/// A client is named by its [`ClientAddress`] for as long as its registration lives, and reaches
/// the node from whatever address it currently holds. This maps the stable name to the volatile
/// address, and only in that direction: an address identifies nobody - it is a place, shared by
/// everyone behind a NAT and reassigned freely - so nothing may resolve a client *from* one.
///
/// Nothing here is specific to a particular transport - it is the mapping every scheme needs once
/// clients stop being identified by the connection they arrived on.
#[derive(Clone, Default)]
pub struct ClientRegistry {
    last_seen: Arc<DashMap<ClientAddress, SocketAddr>>,
}

impl ClientRegistry {
    /// Record where `client` was last seen.
    pub fn refresh(&self, client: ClientAddress, seen_at: SocketAddr) {
        self.last_seen.insert(client, seen_at);
    }

    pub fn last_seen(&self, client: ClientAddress) -> Option<SocketAddr> {
        self.last_seen.get(&client).map(|entry| *entry.value())
    }

    /// Drop what is known about `client`.
    ///
    /// An address is only meaningful while the client still has a session to reach it on, so this
    /// is driven by whatever owns those. Without it the registry grows one entry per client ever
    /// seen and never sheds any.
    pub fn forget(&self, client: ClientAddress) {
        self.last_seen.remove(&client);
    }
}

impl FromIterator<(ClientAddress, SocketAddr)> for ClientRegistry {
    fn from_iter<T: IntoIterator<Item = (ClientAddress, SocketAddr)>>(iter: T) -> Self {
        let registry = ClientRegistry::default();
        for (client, seen_at) in iter {
            registry.refresh(client, seen_at);
        }
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn client(n: u8) -> ClientAddress {
        ClientAddress::from_bytes([n; 20])
    }

    fn addr(n: u8, port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, n)), port)
    }

    /// A client that moves keeps receiving: the newest address wins.
    #[test]
    fn a_client_is_followed_to_its_new_address() {
        let registry = ClientRegistry::default();
        let (first, second) = (addr(4, 51264), addr(5, 41000));

        registry.refresh(client(1), first);
        registry.refresh(client(1), second);

        assert_eq!(registry.last_seen(client(1)), Some(second));
    }

    /// Clients sharing an address are tracked independently.
    ///
    /// This is the NAT case: several clients behind one public IP, or even at the identical socket
    /// address once a port is reassigned. Since nothing resolves a client *from* an address, they
    /// cannot collide.
    /// Forgetting one client leaves the rest alone.
    ///
    /// Driven by session eviction, so this is what stops the registry growing one entry per client
    /// ever seen.
    #[test]
    fn a_forgotten_client_is_gone() {
        let registry = ClientRegistry::default();
        registry.refresh(client(1), addr(4, 51264));
        registry.refresh(client(2), addr(5, 51264));

        registry.forget(client(1));

        assert_eq!(registry.last_seen(client(1)), None);
        assert_eq!(registry.last_seen(client(2)), Some(addr(5, 51264)));
    }

    #[test]
    fn clients_sharing_an_address_do_not_collide() {
        let registry = ClientRegistry::default();
        let shared = addr(4, 51264);

        registry.refresh(client(1), shared);
        registry.refresh(client(2), shared);
        registry.refresh(client(1), addr(6, 51264));

        assert_eq!(registry.last_seen(client(1)), Some(addr(6, 51264)));
        assert_eq!(
            registry.last_seen(client(2)),
            Some(shared),
            "one client moving must not disturb another that shared its address"
        );
    }
}
