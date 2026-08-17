// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The ed25519 client identities announced by authorised network monitor agents.
//!
//! A client websocket session whose registration handshake authenticates one of these identities is
//! an ephemeral monitor session: unmetered, requiring no bandwidth credential and persisting
//! nothing. The gate is the handshake-verified identity and NEVER the connection's source IP, since
//! agents share host ports and run on recycled address pools, while the exemption itself is not
//! confined by the routing filter.
//!
//! Entries are derived from the same authorised-agent set as the node's IP-keyed structures, but
//! keyed by identity rather than by address. Each identity carries the agent entries that announced
//! it, because an agent authorises one entry per address family and both carry its identity: the
//! identity therefore outlives the revocation of any one of them and is dropped only with the last.

use arc_swap::ArcSwap;
use nym_crypto::asymmetric::ed25519;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

/// Thread-safe, lock-free set of the ed25519 client identities announced by authorised network
/// monitor agents.
///
/// Reads are lock-free and happen once per client registration handshake; writes are rare (only on
/// agent authorisation events) and copy the whole map, which is acceptable for a handful of entries.
///
/// Cloning is cheap: it only clones the `Arc`, not the underlying data.
#[derive(Clone, Debug, Default)]
pub(crate) struct AuthorisedMonitorIdentities {
    inner: Arc<ArcSwap<AuthorisedMonitorIdentitiesInner>>,
}

impl AuthorisedMonitorIdentities {
    /// Whether the given handshake-verified client identity belongs to an authorised agent.
    ///
    /// Only the write side needs to know which agent entries announced an identity; a reader asks
    /// this one question, which is the whole of what the client-session gate is handed.
    // read by the client-session gate, which is added separately
    #[allow(dead_code)]
    pub(crate) fn is_announced(&self, identity: &ed25519::PublicKey) -> bool {
        self.inner.load().is_announced(identity)
    }

    /// Record what an agent entry announces, replacing whatever it announced before.
    ///
    /// `None` withdraws the entry's previous claim without touching the other entries of the same
    /// agent, which covers both an agent that announces no identity and one whose announced value
    /// was unusable. Passing a changed identity moves the entry across, so a re-authorisation can
    /// never leave the superseded identity exempt.
    pub(crate) fn set_announced(&self, address: SocketAddr, identity: Option<ed25519::PublicKey>) {
        let mut updated = self.inner.load().as_ref().clone();

        updated.withdraw(address);
        if let Some(identity) = identity {
            updated.announce(address, identity);
        }

        self.inner.store(Arc::new(updated))
    }

    /// Withdraw a revoked agent entry's claim, dropping its identity once no entry announces it.
    pub(crate) fn remove(&self, address: SocketAddr) {
        let mut updated = self.inner.load().as_ref().clone();

        if updated.withdraw(address) {
            self.inner.store(Arc::new(updated))
        }
    }

    /// Forget every announced identity.
    pub(crate) fn reset(&self) {
        self.inner.store(Default::default())
    }
}

#[derive(Clone, Debug, Default)]
struct AuthorisedMonitorIdentitiesInner {
    /// Announced identities, each holding the agent entries that announced it.
    announced: HashMap<ed25519::PublicKey, HashSet<SocketAddr>>,
}

impl AuthorisedMonitorIdentitiesInner {
    fn is_announced(&self, identity: &ed25519::PublicKey) -> bool {
        self.announced.contains_key(identity)
    }

    fn announce(&mut self, address: SocketAddr, identity: ed25519::PublicKey) {
        self.announced
            .entry(identity)
            .or_default()
            .insert(canonical(address));
    }

    /// Drop an agent entry's claim on whichever identity holds it, removing the identity if that
    /// was its last entry. Returns whether anything changed.
    ///
    /// The scan is linear over the identities, which is what keying by identity costs on the write
    /// path: a revocation names only an address. Writes are rare and the map holds one entry per
    /// authorised agent, so this stays cheaper than maintaining a second address-keyed index.
    fn withdraw(&mut self, address: SocketAddr) -> bool {
        let address = canonical(address);

        let Some(identity) = self
            .announced
            .iter()
            .find(|(_, addresses)| addresses.contains(&address))
            .map(|(identity, _)| *identity)
        else {
            return false;
        };

        let addresses = self
            .announced
            .get_mut(&identity)
            .expect("the identity we just found is gone");

        addresses.remove(&address);
        if addresses.is_empty() {
            self.announced.remove(&identity);
        }

        true
    }
}

/// Agent entries are held in canonical form so that an entry announced in one address form is still
/// found when it is revoked in the other.
fn canonical(address: SocketAddr) -> SocketAddr {
    SocketAddr::new(address.ip().to_canonical(), address.port())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::u64_seeded_rng;
    use std::net::{IpAddr, Ipv4Addr};

    // seeded per call: the shared `deterministic_rng` uses one fixed seed, so it would hand every
    // call the same key and quietly pass any test meant to tell two identities apart
    fn identity(seed: u64) -> ed25519::PublicKey {
        *ed25519::KeyPair::new(&mut u64_seeded_rng(seed)).public_key()
    }

    fn v4(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), port)
    }

    fn v6(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V6(Ipv4Addr::new(5, 6, 7, 8).to_ipv6_mapped()), port)
    }

    #[test]
    fn an_announced_identity_is_recognised() {
        let identities = AuthorisedMonitorIdentities::default();
        let identity = identity(1);

        assert!(!identities.is_announced(&identity));

        identities.set_announced(v4(39322), Some(identity));

        assert!(identities.is_announced(&identity));
    }

    // An agent authorises one entry per address family and both carry its identity, so the identity
    // must survive the revocation of either one and be dropped only with the last.
    #[test]
    fn an_identity_announced_by_two_entries_outlives_the_first_revocation() {
        let identities = AuthorisedMonitorIdentities::default();
        let identity = identity(1);

        identities.set_announced(v4(39322), Some(identity));
        identities.set_announced(v6(39322), Some(identity));

        identities.remove(v4(39322));
        assert!(identities.is_announced(&identity));

        identities.remove(v6(39322));
        assert!(!identities.is_announced(&identity));
    }

    // A re-authorisation carrying a changed identity must not leave the superseded one exempt.
    #[test]
    fn a_changed_identity_supersedes_the_previous_one() {
        let identities = AuthorisedMonitorIdentities::default();
        let old = identity(1);
        let new = identity(2);

        identities.set_announced(v4(39322), Some(old));
        identities.set_announced(v4(39322), Some(new));

        assert!(!identities.is_announced(&old));
        assert!(identities.is_announced(&new));
    }

    // An agent that stops announcing an identity withdraws its claim, but only its own: the other
    // entries of the same agent keep the identity alive.
    #[test]
    fn announcing_no_identity_withdraws_only_that_entrys_claim() {
        let identities = AuthorisedMonitorIdentities::default();
        let identity = identity(1);

        identities.set_announced(v4(39322), Some(identity));
        identities.set_announced(v6(39322), Some(identity));

        identities.set_announced(v4(39322), None);
        assert!(identities.is_announced(&identity));

        identities.set_announced(v6(39322), None);
        assert!(!identities.is_announced(&identity));
    }

    // Entries must be found under the canonical form regardless of which form each call used,
    // otherwise a revocation announced in the other form would silently leave the identity exempt.
    #[test]
    fn addresses_match_across_ipv4_and_ipv4_mapped_forms() {
        let identities = AuthorisedMonitorIdentities::default();
        let identity = identity(1);

        let plain = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 39322);
        let mapped = SocketAddr::new(
            IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped()),
            39322,
        );

        identities.set_announced(mapped, Some(identity));
        identities.remove(plain);

        assert!(!identities.is_announced(&identity));
    }

    // Agents sharing a host are disambiguated by port, so revoking one must not withdraw another's
    // claim on the same identity.
    #[test]
    fn entries_on_one_host_are_disambiguated_by_port() {
        let identities = AuthorisedMonitorIdentities::default();
        let identity = identity(1);

        identities.set_announced(v4(39322), Some(identity));
        identities.set_announced(v4(39323), Some(identity));

        identities.remove(v4(39322));

        assert!(identities.is_announced(&identity));
    }

    #[test]
    fn reset_forgets_every_identity() {
        let identities = AuthorisedMonitorIdentities::default();
        let first = identity(1);
        let second = identity(2);

        identities.set_announced(v4(39322), Some(first));
        identities.set_announced(v6(39322), Some(second));

        identities.reset();

        assert!(!identities.is_announced(&first));
        assert!(!identities.is_announced(&second));
    }

    // Revoking an entry nobody announced must not disturb the map.
    #[test]
    fn removing_an_unknown_address_is_a_no_op() {
        let identities = AuthorisedMonitorIdentities::default();
        let identity = identity(1);

        identities.set_announced(v4(39322), Some(identity));
        identities.remove(v4(39999));

        assert!(identities.is_announced(&identity));
    }
}
