// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The challenges this node issues, and the ones it has already accepted.
//!
//! A mixnet registration binds a name to a session, and a signature over the client's own material
//! alone says nothing about when it was made - a copy of it, presented by anyone, binds the same
//! name. So the node answers the first message with a nonce and only accepts a finalisation that
//! signs it, once.
//!
//! # Nothing is stored when a nonce is issued
//!
//! The opening request is unsigned and costs nothing to send, so a node that remembered every
//! nonce it handed out could be made to remember any number of them. Instead a nonce carries its
//! own proof: when it was issued, and a MAC under a secret only this node holds over that time and
//! the identity it was issued to. Redeeming recomputes the MAC, so a nonce is good only for the
//! identity that asked for it, and only for [`NONCE_TTL`].
//!
//! What is stored is the nonces already *spent*, so that none is accepted twice. That set grows
//! only with finalisations that carry a valid signature over a live nonce, each of which binds a
//! session, so it is bounded by real registrations rather than by what anyone cares to send.
//!
//! The secret lives for as long as the node does, so a restart invalidates the nonces it had
//! issued. Those are at most [`NONCE_TTL`] old, and a client refused one simply asks again.
//!
//! # Not tied to a session
//!
//! Deliberately: nothing on the wire names a session, which is what lets the exchange survive a
//! transport that does not preserve one - a bridge, say. The price is that a finalisation which
//! leaks is usable by whoever holds it until it expires, so the TTL is the only thing bounding
//! that and is kept short.

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use nym_crypto::asymmetric::ed25519;
use nym_registration_common::mixnet::RegistrationNonce;
use rand::RngExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

/// How long a nonce stays redeemable.
///
/// Short, because it is the only bound on a leaked finalisation: a client that has just been
/// answered is about to reply, so this only has to cover one round trip.
pub const NONCE_TTL: Duration = Duration::from_secs(15);

/// Bytes of a nonce taken by its issue time; the MAC fills the rest.
const ISSUED_LEN: usize = size_of::<u64>();
const MAC_LEN: usize = size_of::<RegistrationNonce>() - ISSUED_LEN;

const NONCE_TTL_MS: u64 = NONCE_TTL.as_millis() as u64;

/// What a nonce is made of.
///
/// Only this node reads it: to a client, and on the wire, a nonce is an opaque
/// [`RegistrationNonce`], so the layout can change without anyone else noticing.
struct NonceParts {
    /// Milliseconds since [`RegistrationNonces::epoch`].
    issued_ms: u64,

    /// Over the identity the nonce was issued to and [`Self::issued_ms`].
    mac: [u8; MAC_LEN],
}

impl From<NonceParts> for RegistrationNonce {
    fn from(parts: NonceParts) -> Self {
        let mut nonce: RegistrationNonce = Default::default();
        nonce[..ISSUED_LEN].copy_from_slice(&parts.issued_ms.to_be_bytes());
        nonce[ISSUED_LEN..].copy_from_slice(&parts.mac);
        nonce
    }
}

impl From<&RegistrationNonce> for NonceParts {
    fn from(nonce: &RegistrationNonce) -> Self {
        let mut issued = [0u8; ISSUED_LEN];
        issued.copy_from_slice(&nonce[..ISSUED_LEN]);

        let mut mac = [0u8; MAC_LEN];
        mac.copy_from_slice(&nonce[ISSUED_LEN..]);

        NonceParts {
            issued_ms: u64::from_be_bytes(issued),
            mac,
        }
    }
}

#[derive(Clone)]
pub struct RegistrationNonces {
    /// What nonces are MACed under. Never leaves this node.
    secret: Arc<[u8; 32]>,

    /// What issue times are measured from. Monotonic, so no clock adjustment can make a nonce look
    /// fresher than it is.
    epoch: Instant,

    /// Nonces already redeemed, against when they were issued, kept until they would have expired
    /// anyway.
    spent: Arc<DashMap<RegistrationNonce, u64>>,
}

impl RegistrationNonces {
    pub fn new() -> Self {
        RegistrationNonces {
            secret: Arc::new(rand::rng().random()),
            epoch: Instant::now(),
            spent: Arc::new(DashMap::new()),
        }
    }

    /// Mint a nonce that only `identity` can redeem. Stores nothing.
    pub fn issue(&self, identity: &ed25519::PublicKey) -> RegistrationNonce {
        self.issue_at(identity, self.now_ms())
    }

    fn issue_at(&self, identity: &ed25519::PublicKey, issued_ms: u64) -> RegistrationNonce {
        NonceParts {
            issued_ms,
            mac: self.mac(identity, issued_ms),
        }
        .into()
    }

    /// Milliseconds since [`Self::epoch`], which is what a nonce records as its issue time.
    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    /// The MAC a nonce issued to `identity` at `issued_ms` carries.
    fn mac(&self, identity: &ed25519::PublicKey, issued_ms: u64) -> [u8; MAC_LEN] {
        let hash = blake3::Hasher::new_keyed(&self.secret)
            .update(&identity.to_bytes())
            .update(&issued_ms.to_be_bytes())
            .finalize();

        let mut mac = [0u8; MAC_LEN];
        mac.copy_from_slice(&hash.as_bytes()[..MAC_LEN]);
        mac
    }

    fn redeem_at(
        &self,
        identity: &ed25519::PublicKey,
        nonce: &RegistrationNonce,
        now_ms: u64,
    ) -> bool {
        let parts = NonceParts::from(nonce);

        // if this failed, the nonce was issued "in the future" so it's not ours for sure
        let Some(age_ms) = now_ms.checked_sub(parts.issued_ms) else {
            return false;
        };
        if age_ms > NONCE_TTL_MS {
            return false;
        }

        if !bool::from(self.mac(identity, parts.issued_ms).ct_eq(&parts.mac)) {
            return false;
        }

        // through the entry API, so two concurrent finalisations cannot both find it unspent
        match self.spent.entry(*nonce) {
            Entry::Occupied(_) => false,
            Entry::Vacant(slot) => {
                slot.insert(parts.issued_ms);
                true
            }
        }
    }

    /// Spend a nonce on behalf of `identity`, which succeeds at most once, only for the identity
    /// the nonce was issued to, and only within [`NONCE_TTL`] of its issue.
    pub fn redeem(&self, identity: &ed25519::PublicKey, nonce: &RegistrationNonce) -> bool {
        self.redeem_at(identity, nonce, self.now_ms())
    }

    fn remove_stale_at(&self, now_ms: u64) -> u64 {
        let before = self.spent.len();
        self.spent
            .retain(|_, issued_ms| now_ms.saturating_sub(*issued_ms) <= NONCE_TTL_MS);

        (before - self.spent.len()) as u64
    }

    /// Forget spent nonces that have expired.
    ///
    /// Only reclaims memory: [`Self::redeem`] already refuses an expired nonce, so a sweep that
    /// never ran would cost space rather than safety.
    pub fn remove_stale(&self) -> u64 {
        self.remove_stale_at(self.now_ms())
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
    use nym_test_utils::helpers::deterministic_rng;

    /// Past the TTL, by enough that rounding cannot matter.
    const EXPIRED_MS: u64 = NONCE_TTL_MS + 1_000;

    fn identities() -> (ed25519::PublicKey, ed25519::PublicKey) {
        let mut rng = deterministic_rng();
        (
            *ed25519::KeyPair::new(&mut rng).public_key(),
            *ed25519::KeyPair::new(&mut rng).public_key(),
        )
    }

    #[test]
    fn a_nonce_is_redeemable_once() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue(&client);

        assert!(nonces.redeem(&client, &nonce));
        assert!(!nonces.redeem(&client, &nonce));
    }

    #[test]
    fn a_nonce_nobody_issued_is_refused() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        assert!(!nonces.redeem(&client, &[0u8; 32]));
    }

    #[test]
    fn a_nonce_is_only_good_for_the_identity_it_was_issued_to() {
        let (client, other) = identities();
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue(&client);

        assert!(!nonces.redeem(&other, &nonce));
        // and refusing it for someone else did not spend it
        assert!(nonces.redeem(&client, &nonce));
    }

    #[test]
    fn a_nonce_from_another_node_is_refused() {
        let (client, _) = identities();
        let nonce = RegistrationNonces::new().issue(&client);

        assert!(!RegistrationNonces::new().redeem(&client, &nonce));
    }

    #[test]
    fn a_tampered_issue_time_is_refused() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        let mut parts = NonceParts::from(&nonces.issue_at(&client, 0));

        // claim it was issued later than it was, to stretch its lifetime
        parts.issued_ms = EXPIRED_MS;

        assert!(!nonces.redeem_at(&client, &parts.into(), EXPIRED_MS));
    }

    #[test]
    fn a_nonce_from_the_future_is_refused() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue_at(&client, EXPIRED_MS);

        assert!(!nonces.redeem_at(&client, &nonce, 0));
    }

    #[test]
    fn an_expired_nonce_is_refused() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue_at(&client, 0);

        assert!(!nonces.redeem_at(&client, &nonce, EXPIRED_MS));
    }

    #[test]
    fn issuing_stores_nothing() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        for _ in 0..100 {
            nonces.issue(&client);
        }

        assert!(nonces.spent.is_empty());
    }

    #[test]
    fn a_spent_nonce_survives_a_sweep_while_it_could_still_be_replayed() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue_at(&client, 0);
        assert!(nonces.redeem_at(&client, &nonce, 0));

        assert_eq!(nonces.remove_stale_at(NONCE_TTL_MS), 0);
        assert!(!nonces.redeem_at(&client, &nonce, NONCE_TTL_MS));
    }

    #[test]
    fn a_spent_nonce_is_swept_once_it_has_expired() {
        let (client, _) = identities();
        let nonces = RegistrationNonces::new();
        let nonce = nonces.issue_at(&client, 0);
        assert!(nonces.redeem_at(&client, &nonce, 0));

        assert_eq!(nonces.remove_stale_at(EXPIRED_MS), 1);
        // and with it gone, expiry alone keeps it from being replayed
        assert!(!nonces.redeem_at(&client, &nonce, EXPIRED_MS));
    }
}
