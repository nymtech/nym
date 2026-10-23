// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The challenges this node has issued and is still willing to accept.
//!
//! A mixnet registration binds a name to a session, and a signature over the client's own material
//! alone says nothing about when it was made - a copy of it, presented by anyone, binds the same
//! name. So the node answers the first message with a nonce and only accepts a finalisation that
//! signs it, once.
//!
//! Deliberately *not* tied to the session the nonce was issued to: nothing on the wire names a
//! session, which is what lets the exchange survive a transport that does not preserve one - a
//! bridge, say. The price is that a finalisation which leaks is usable by whoever holds it until
//! it expires, so the TTL is the only thing bounding that and is kept short.

use dashmap::DashMap;
use nym_registration_common::mixnet::RegistrationNonce;
use rand::RngExt;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long a nonce stays redeemable.
///
/// Short, because it is the only bound on a leaked finalisation: a client that has just been
/// answered is about to reply, so this only has to cover one round trip.
pub const NONCE_TTL: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct RegistrationNonces {
    /// Nonces issued and not yet redeemed, against when they were issued. Monotonic, so no clock
    /// adjustment can make one look fresher than it is.
    issued: Arc<DashMap<RegistrationNonce, Instant>>,
}

impl RegistrationNonces {
    pub fn new() -> Self {
        RegistrationNonces {
            issued: Arc::new(DashMap::new()),
        }
    }

    /// Mint a nonce and remember it until it is redeemed or expires.
    pub fn issue(&self) -> RegistrationNonce {
        let nonce: RegistrationNonce = rand::rng().random();
        self.issued.insert(nonce, Instant::now());
        nonce
    }

    /// Spend a nonce, which succeeds at most once for any given nonce.
    ///
    /// Removing before checking the age is deliberate: an expired nonce is never redeemable again,
    /// so there is nothing to gain by leaving it behind, and taking it out closes the window in
    /// which two concurrent finalisations could both find it.
    pub fn redeem(&self, nonce: &RegistrationNonce) -> bool {
        let Some((_, issued)) = self.issued.remove(nonce) else {
            return false;
        };

        issued.elapsed() <= NONCE_TTL
    }

    /// Drop nonces nobody came back for.
    ///
    /// Only reclaims memory: [`Self::redeem`] already refuses an expired nonce, so a sweep that
    /// never ran would cost space rather than safety.
    pub fn remove_stale(&self) -> u64 {
        let before = self.issued.len();
        self.issued
            .retain(|_, issued| issued.elapsed() <= NONCE_TTL);

        (before - self.issued.len()) as u64
    }
}

impl Default for RegistrationNonces {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nonce_is_redeemable_once() {
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue();

        assert!(nonces.redeem(&nonce));
        assert!(!nonces.redeem(&nonce));
    }

    #[test]
    fn a_nonce_nobody_issued_is_refused() {
        let nonces = RegistrationNonces::new();
        assert!(!nonces.redeem(&[0u8; 32]));
    }

    #[test]
    fn every_nonce_is_its_own() {
        let nonces = RegistrationNonces::new();
        assert_ne!(nonces.issue(), nonces.issue());
    }

    /// Pretend a nonce was issued `by` ago, since the TTL is far too long to wait out.
    fn backdate(nonces: &RegistrationNonces, nonce: &RegistrationNonce, by: Duration) {
        let issued = Instant::now() - by;
        nonces.issued.insert(*nonce, issued);
    }

    #[test]
    fn a_fresh_nonce_survives_a_sweep() {
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue();

        assert_eq!(nonces.remove_stale(), 0);
        assert!(nonces.redeem(&nonce));
    }

    #[test]
    fn a_swept_nonce_is_no_longer_redeemable() {
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue();
        backdate(&nonces, &nonce, NONCE_TTL + Duration::from_secs(1));

        assert_eq!(nonces.remove_stale(), 1);
        assert!(!nonces.redeem(&nonce));
    }

    /// The sweep only reclaims memory, so an expired nonce is refused whether or not one ran.
    #[test]
    fn an_expired_nonce_is_refused_without_a_sweep() {
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue();
        backdate(&nonces, &nonce, NONCE_TTL + Duration::from_secs(1));

        assert!(!nonces.redeem(&nonce));
    }
}
