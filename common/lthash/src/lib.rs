#![cfg_attr(not(test), no_std)]

//! LtHash: a homomorphic (incremental) multiset hash, generic over a XOF.
//!
//! Each element is expanded by the XOF into 1024 little-endian `u16` lanes; the
//! digest is the component-wise wrapping sum of every inserted element's lanes.
//! [`LtHash::add`] and [`LtHash::subtract`] mutate the digest, which commits to the
//! *multiset* of elements independent of order and supports O(1) updates;
//! [`LtHash::out`] collapses it to a compact one-way commitment for proving/comparing.
//!
//! Security: instantiate with a cryptographic XOF (this crate ships [`LtHash16`]
//! over BLAKE3; `sha3::Shake256` is used in tests). A naive XOR/sum would be
//! forgeable. Multiset caveat: adding the *same* element 2^16 times wraps to
//! zero - not reachable in append-only / 0-or-1-multiplicity usage.
//!
//! # References
//! * <https://eprint.iacr.org/2019/227>: Securing Update Propagation with Homomorphic Hashing
//! * <https://github.com/facebook/folly/blob/main/folly/crypto/LtHash.cpp>: C++ implementation of the LtHash used at Facebook.

use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use digest::{Digest, ExtendableOutput, Output, Update, XofReader};

const ELEMENTS: usize = 1024;

/// Serialized digest size in bytes (1024 lanes * 2 bytes).
pub const DIGEST_LEN: usize = ELEMENTS * 2;

/// An LtHash digest, generic over the expansion XOF `H`.
pub struct LtHash<H> {
    state: [u16; ELEMENTS],
    _xof: PhantomData<H>,
}

/// LtHash instantiated with BLAKE3 - the production digest type.
pub type LtHash16 = LtHash<blake3::Hasher>;

impl<H> LtHash<H> {
    /// The empty digest (identity element).
    pub const fn new() -> Self {
        Self {
            state: [0u16; ELEMENTS],
            _xof: PhantomData,
        }
    }

    /// Serialize to the canonical little-endian byte form.
    pub fn to_bytes(&self) -> [u8; DIGEST_LEN] {
        let mut out = [0u8; DIGEST_LEN];
        for i in 0..ELEMENTS {
            let b = self.state[i].to_le_bytes();
            out[2 * i] = b[0];
            out[2 * i + 1] = b[1];
        }
        out
    }

    /// Deserialize from the canonical byte form.
    pub fn from_bytes(bytes: &[u8; DIGEST_LEN]) -> Self {
        let mut lanes = [0u16; ELEMENTS];
        for i in 0..ELEMENTS {
            lanes[i] = u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
        }
        Self {
            state: lanes,
            _xof: PhantomData,
        }
    }
}

// if serialiser is human-readable (e.g. json) - use base64,
// otherwise (e.g. bincone/proto, etc.) use the raw bytes.
#[cfg(feature = "serde")]
impl<H> serde::Serialize for LtHash<H> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde_with::{Bytes, IfIsHumanReadable, SerializeAs, base64::Base64};
        IfIsHumanReadable::<Base64, Bytes>::serialize_as(&self.to_bytes(), serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, H> serde::Deserialize<'de> for LtHash<H> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde_with::{Bytes, DeserializeAs, IfIsHumanReadable, base64::Base64};
        let bytes: [u8; DIGEST_LEN] =
            IfIsHumanReadable::<Base64, Bytes>::deserialize_as(deserializer)?;
        Ok(LtHash::from_bytes(&bytes))
    }
}

impl<H: Default + Update + ExtendableOutput> LtHash<H> {
    fn expand(element: &[u8]) -> [u16; ELEMENTS] {
        let mut hasher = H::default();
        let mut bytes = [0u8; DIGEST_LEN];
        let mut lanes = [0u16; ELEMENTS];

        hasher.update(element);
        hasher.finalize_xof().read(&mut bytes);

        for i in 0..ELEMENTS {
            lanes[i] = u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
        }
        lanes
    }

    /// Add an element to the multiset.
    #[allow(clippy::needless_range_loop)]
    pub fn add(&mut self, element: &[u8]) {
        let e = Self::expand(element);
        for i in 0..ELEMENTS {
            self.state[i] = self.state[i].wrapping_add(e[i]);
        }
    }

    /// Remove a previously-inserted element from the multiset.
    #[allow(clippy::needless_range_loop)]
    pub fn subtract(&mut self, element: &[u8]) {
        let e = Self::expand(element);
        for i in 0..ELEMENTS {
            self.state[i] = self.state[i].wrapping_sub(e[i]);
        }
    }
}

impl<H: Default + Digest> LtHash<H> {
    /// Fixed-size one-way hash of the full state, for exposing/proving/comparing a
    /// digest compactly (32 bytes under the BLAKE3 [`LtHash16`]). NOT homomorphic - use
    /// only for comparison, never as the accumulator.
    pub fn out(&self) -> Output<H> {
        let mut hasher = H::default();
        for &val in &self.state {
            hasher.update(val.to_le_bytes());
        }
        hasher.finalize()
    }
}

impl<H> Default for LtHash<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H> Clone for LtHash<H> {
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            _xof: PhantomData,
        }
    }
}

impl<H> PartialEq for LtHash<H> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<T> Hash for LtHash<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.hash(state);
    }
}

impl<H> Eq for LtHash<H> {}

impl<H> core::fmt::Debug for LtHash<H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LtHash(")?;
        for b in self.to_bytes() {
            write!(f, "{b:02x}")?;
        }
        write!(f, ")")
    }
}

impl<H> core::ops::Add for LtHash<H> {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        for i in 0..ELEMENTS {
            self.state[i] = self.state[i].wrapping_add(rhs.state[i]);
        }
        self
    }
}

impl<H> core::ops::Sub for LtHash<H> {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self {
        for i in 0..ELEMENTS {
            self.state[i] = self.state[i].wrapping_sub(rhs.state[i]);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Same invariants must hold for any XOF - run against blake3 (prod) and sha3 (dev).
    fn check_invariants<H: Default + Update + ExtendableOutput>() {
        // insert then remove is the identity
        let mut h = LtHash::<H>::new();
        h.add(b"alice");
        h.subtract(b"alice");
        assert_eq!(h, LtHash::<H>::new(), "insert+remove must be identity");

        // order independence
        let mut ab = LtHash::<H>::new();
        ab.add(b"alice");
        ab.add(b"bob");
        let mut ba = LtHash::<H>::new();
        ba.add(b"bob");
        ba.add(b"alice");
        assert_eq!(ab, ba, "multiset hash must be order-independent");

        // homomorphism: combine == union of singletons
        let mut only_a = LtHash::<H>::new();
        only_a.add(b"alice");
        let mut only_b = LtHash::<H>::new();
        only_b.add(b"bob");
        assert_eq!(
            only_a.clone() + only_b.clone(),
            ab,
            "Add must equal the union"
        );

        // Sub inverts Add
        assert_eq!(ab.clone() - only_b, only_a, "Sub must undo Add");

        // byte round-trip
        assert_eq!(
            LtHash::<H>::from_bytes(&ab.to_bytes()),
            ab,
            "byte round-trip"
        );

        // distinct elements give distinct digests
        assert_ne!(only_a, LtHash::<H>::new());
    }

    // human-readable = false
    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trips_through_bincode() {
        let mut lt = LtHash16::new();
        lt.add(b"alice");
        lt.add(b"bob");

        let bytes = bincode::serialize(&lt).unwrap();
        let restored: LtHash16 = bincode::deserialize(&bytes).unwrap();
        assert_eq!(lt, restored);
    }

    // human-readable = true
    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trips_through_serde_json() {
        let mut lt = LtHash16::new();
        lt.add(b"alice");
        lt.add(b"bob");

        let bytes = serde_json::to_vec(&lt).unwrap();
        let restored: LtHash16 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(lt, restored);
    }

    #[test]
    fn invariants_blake3() {
        check_invariants::<blake3::Hasher>();
    }

    #[test]
    fn invariants_sha3() {
        check_invariants::<sha3::Shake256>();
    }

    #[test]
    fn empty_digest_is_zero() {
        assert_eq!(LtHash16::new().to_bytes(), [0u8; DIGEST_LEN]);
    }

    #[test]
    fn out_equals_blake3_of_state() {
        let mut lt = LtHash16::new();
        lt.add(b"alice");
        lt.add(b"bob");
        // the generic Digest collapse must be exactly BLAKE3 over the canonical state bytes
        let out = lt.out();
        let hash = blake3::hash(&lt.to_bytes());
        let via_out: &[u8] = out.as_slice();
        let via_blake3: &[u8] = hash.as_bytes();
        assert_eq!(via_out, via_blake3);
    }

    #[test]
    fn out_is_stable_and_distinguishes_states() {
        let mut a = LtHash16::new();
        a.add(b"alice");
        let mut a2 = LtHash16::new();
        a2.add(b"alice");
        let mut b = LtHash16::new();
        b.add(b"bob");

        // deterministic, and equal multisets collapse equally
        assert_eq!(a.out(), a.clone().out());
        assert_eq!(a.out(), a2.out());
        // distinct multisets collapse differently
        assert_ne!(a.out(), b.out());
    }
}
