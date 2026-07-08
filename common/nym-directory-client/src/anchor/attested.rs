// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::DirectoryClientError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::chain;
use futures::future::join_all;
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use nym_network_defaults::default_directory_attestation_sources;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::hash::AppHash;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use tokio::sync::Mutex;

/// Domain-separation tag for [`digest_snapshot_signing_payload`], so a snapshot
/// signature can never be interpreted as a `node_signing_payload` signature (which
/// carries no tag of its own), even for a signer whose identity key is used for both.
const DIGEST_SNAPSHOT_DOMAIN_TAG: &[u8] = b"nym-directory-digest-snapshot-v1";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DigestSnapshot {
    /// The chain this attestation is scoped to, so a signature cannot be replayed
    /// against a different chain.
    chain_id: chain::Id,

    /// The directory contract this attestation is scoped to, so a signature cannot be
    /// replayed against a different contract instance.
    directory_contract: AccountId,

    /// The block height every other field attests to.
    height: Height,

    /// The block `app_hash` at `height` - the ICS23 fallback root for single-entry reads.
    #[serde(with = "cosmrs::tendermint::serializers::apphash")]
    app_hash: AppHash,

    /// The directory contract's LtHash accumulator at `height`.
    accumulator: LtHash16,

    /// Hash over the current `NodeId -> ed25519 identity` mapping at `height`
    /// (see [`crate::verify::node_identities_hash`]).
    node_identities_hash: [u8; 32],
}

impl Hash for DigestSnapshot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.chain_id.hash(state);
        self.directory_contract.as_ref().hash(state);
        self.height.hash(state);
        self.app_hash.as_ref().hash(state);
        self.accumulator.hash(state);
        self.node_identities_hash.hash(state);
    }
}

impl DigestSnapshot {
    pub(crate) fn signing_payload(&self) -> Vec<u8> {
        digest_snapshot_signing_payload(
            self.chain_id.as_ref(),
            &self.directory_contract,
            self.height,
            &self.app_hash,
            &self.accumulator,
            &self.node_identities_hash,
        )
    }
}

/// A [`DigestSnapshot`] as published by a nym-api (or a nym-node), together with its signer and
/// signature over the snapshot's canonical signing payload.
#[derive(Clone, Serialize, Deserialize)]
pub struct SignedDigestSnapshot {
    snapshot: DigestSnapshot,

    signer: ed25519::PublicKey,

    signature: ed25519::Signature,
}

impl SignedDigestSnapshot {
    /// Whether this attestation is trustworthy on its own: `signer` is in `trusted`,
    /// the snapshot is scoped to `chain_id` and `contract`, and the signature verifies
    /// over the canonical signing payload. Says nothing about quorum - that is the
    /// anchor's job, counting distinct signers across many valid attestations like this
    /// one. Mirrors `node_signature_verifies`.
    pub(crate) fn verify(
        &self,
        trusted: &HashSet<ed25519::PublicKey>,
        chain_id: &chain::Id,
        contract: &AccountId,
    ) -> bool {
        if !trusted.contains(&self.signer) {
            return false;
        }
        if &self.snapshot.chain_id != chain_id || &self.snapshot.directory_contract != contract {
            return false;
        }
        self.signer
            .verify(self.snapshot.signing_payload(), &self.signature)
            .is_ok()
    }
}

/// A source of nym-api-signed directory snapshots, so the anchor is independent of any
/// particular transport and can be exercised with a mock.
#[async_trait]
pub trait AttestationSource {
    /// This source's ed25519 identity key.
    fn identity(&self) -> ed25519::PublicKey;

    /// This source's latest signed snapshot.
    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, DirectoryClientError>;

    /// This source's signed snapshot at a specific height, if still within its
    /// retained window.
    async fn snapshot_at(
        &self,
        height: Height,
    ) -> Result<SignedDigestSnapshot, DirectoryClientError>;
}

/// A quorum-agreed `app_hash`, digest accumulator, and node-identity hash for a
/// specific height - the trusted output of [`AttestedTrustAnchor::reach_quorum`].
#[derive(Clone, Debug)]
struct TrustedSnapshot {
    app_hash: AppHash,
    accumulator: LtHash16,
    node_identities_hash: [u8; 32],
}

impl TrustedSnapshot {
    fn from_snapshot(snapshot: DigestSnapshot) -> Self {
        Self {
            app_hash: snapshot.app_hash,
            accumulator: snapshot.accumulator,
            node_identities_hash: snapshot.node_identities_hash,
        }
    }
}

struct AttestedTrustAnchorState {
    snapshots: BTreeMap<Height, TrustedSnapshot>,
    latest: Option<Height>,
}

/// Parses identity keys of attestation sources set in the env into the anchor's native representation.
#[allow(clippy::expect_used)]
fn default_trusted_signers() -> HashSet<ed25519::PublicKey> {
    default_directory_attestation_sources()
        .iter()
        .map(|source| {
            ed25519::PublicKey::from_base58_string(&source.identity_ed25519_bs58)
                .expect("compiled-in default trusted signer key must be valid")
        })
        .collect()
}

/// A [`DirectoryTrustAnchor`](crate::anchor::DirectoryTrustAnchor) backed by a K-of-N
/// quorum of nym-api identity keys signing directory snapshots, rather than a root key
/// or a light-client checkpoint.
pub struct AttestedTrustAnchor<S> {
    sources: Vec<S>,
    trusted_signers: HashSet<ed25519::PublicKey>,
    quorum: usize,
    chain_id: chain::Id,
    directory_contract: AccountId,

    // we only need Mutex to be able to take &self without mutable reference
    // there's no concurrent access anywhere
    state: Mutex<AttestedTrustAnchorState>,
}

impl<S> AttestedTrustAnchor<S> {
    /// Constructs the anchor with a caller-supplied trust root. Rejects a degenerate
    /// quorum (`quorum == 0` or `quorum > trusted_signers.len()`) - no network call is
    /// made, so this cannot fail for any other reason.
    pub fn new(
        sources: Vec<S>,
        trusted_signers: HashSet<ed25519::PublicKey>,
        quorum: usize,
        chain_id: chain::Id,
        directory_contract: AccountId,
    ) -> Result<Self, DirectoryClientError> {
        if quorum == 0 || quorum > trusted_signers.len() {
            return Err(DirectoryClientError::InvalidQuorumConfig {
                quorum,
                signers: trusted_signers.len(),
            });
        }

        Ok(Self {
            sources,
            trusted_signers,
            quorum,
            chain_id,
            directory_contract,
            state: Mutex::new(AttestedTrustAnchorState {
                snapshots: BTreeMap::new(),
                latest: None,
            }),
        })
    }

    /// A simple majority (more than half) of `signer_count` - the quorum policy used
    /// by [`Self::with_default_anchor`]. Expressing the default quorum as a function
    /// of the signer set's size, rather than a separately hardcoded number, means
    /// growing that set (e.g. mainnet's third nym-api gaining a key) automatically
    /// moves the default from 2-of-2 to 2-of-3 with no code change anywhere.
    pub fn majority_quorum(signer_count: usize) -> usize {
        signer_count / 2 + 1
    }

    /// Constructs the anchor using the compiled-in default trust root -
    /// [`nym_network_defaults::mainnet::DIRECTORY_ATTESTATION_SOURCES`]' identity keys,
    /// requiring [`Self::majority_quorum`] of them to agree. This is the common case,
    /// since most deployments have no reason to distrust Nym SA's own instances;
    /// callers who do, or who are not on mainnet, should use [`Self::new`] directly.
    pub fn with_default_anchor(
        sources: Vec<S>,
        chain_id: chain::Id,
        directory_contract: AccountId,
    ) -> Result<Self, DirectoryClientError> {
        let trusted_signers = default_trusted_signers();
        let quorum = Self::majority_quorum(trusted_signers.len());
        Self::new(
            sources,
            trusted_signers,
            quorum,
            chain_id,
            directory_contract,
        )
    }

    /// Filters `candidates` to valid attestations (see
    /// [`SignedDigestSnapshot::verify`]), groups the survivors by
    /// `(height, app_hash, accumulator, node_identities_hash)`, and accepts the *first*
    /// group (in `candidates`' own order) to reach `quorum` distinct signers.
    /// `agreed` in the error case is the largest distinct-signer count seen
    /// across any single group, so callers can see how close the quorum came.
    fn reach_quorum(
        &self,
        candidates: Vec<SignedDigestSnapshot>,
    ) -> Result<(Height, TrustedSnapshot), DirectoryClientError> {
        // map between returned snapshot and signers which attested it
        let mut groups: HashMap<DigestSnapshot, HashSet<ed25519::PublicKey>> = HashMap::new();

        for candidate in candidates {
            // disregard any inconsistent responses
            if !candidate.verify(
                &self.trusted_signers,
                &self.chain_id,
                &self.directory_contract,
            ) {
                continue;
            }

            let snapshot = candidate.snapshot;
            let entry = groups.entry(snapshot.clone()).or_default();
            entry.insert(candidate.signer);

            if entry.len() >= self.quorum {
                return Ok((snapshot.height, TrustedSnapshot::from_snapshot(snapshot)));
            }
        }

        let best_agreed = groups.values().map(|s| s.len()).max().unwrap_or(0);

        Err(DirectoryClientError::QuorumNotReached {
            needed: self.quorum,
            agreed: best_agreed,
        })
    }
}

impl<S> AttestedTrustAnchor<S>
where
    S: AttestationSource + Sync,
{
    /// Queries sources' [`AttestationSource::latest_snapshot`] in pseudorandom order and
    /// returns the first successful response, untrusted at this point - just a height
    /// hint. See [`Self::refresh`].
    async fn first_latest_snapshot(&self) -> Result<SignedDigestSnapshot, DirectoryClientError> {
        // iterate through our sources in pseudorandom order and retrieve the latest snapshot from one of them
        let mut indices: Vec<_> = (0..self.sources.len()).collect();
        let mut rng = rand::thread_rng();
        indices.shuffle(&mut rng);

        for i in indices {
            if let Ok(snapshot) = self.sources[i].latest_snapshot().await {
                return Ok(snapshot);
            }
        }

        Err(DirectoryClientError::QuorumNotReached {
            needed: self.quorum,
            agreed: 0,
        })
    }

    /// Discovers and pins the quorum-agreed latest snapshot: seeds a height from the
    /// first source that answers [`AttestationSource::latest_snapshot`] (untrusted -
    /// a lying seed only wastes a round-trip, since acceptance still requires
    /// `quorum` distinct trusted signers agreeing below), then asks every source's
    /// [`AttestationSource::snapshot_at`] that same height, and
    /// reaches quorum over the seed plus all of those responses. Pinning to one
    /// concrete, already-observed height rather than comparing every source's own
    /// independent "latest" avoids splitting honest sources across a cadence boundary.
    pub async fn refresh(&self) -> Result<Height, DirectoryClientError> {
        let seed = self.first_latest_snapshot().await?;
        let seed_signer = seed.signer;
        let height = seed.snapshot.height;

        let mut candidates = vec![seed];
        candidates.extend(
            join_all(
                self.sources
                    .iter()
                    .filter(|s| s.identity() != seed_signer)
                    .map(|s| s.snapshot_at(height)),
            )
            .await
            .into_iter()
            .filter_map(Result::ok),
        );

        let (height, trusted) = self.reach_quorum(candidates)?;

        let mut state = self.state.lock().await;
        state.snapshots.insert(height, trusted);
        state.latest = Some(height);
        Ok(height)
    }

    /// The cached latest quorum-agreed height, or [`Self::refresh`] if none is cached
    /// yet.
    pub async fn latest_snapshot_height(&self) -> Result<Height, DirectoryClientError> {
        if let Some(height) = self.state.lock().await.latest {
            return Ok(height);
        }

        self.refresh().await
    }

    /// The quorum-agreed snapshot for a specific height a caller already has
    /// independent reason to believe is real (not seeded via [`Self::refresh`]) -
    /// served from cache if present, otherwise fetched fresh from every source's
    /// [`AttestationSource::snapshot_at`]. Verifies the quorum actually agreed on the
    /// *requested* height - a source could otherwise return a validly-signed
    /// attestation for the wrong one. Because `height` only ever comes from a real
    /// observed snapshot, a height the quorum cannot confirm has one coherent meaning,
    /// [`DirectoryClientError::NoQuorumSnapshotForHeight`], whether that is because it
    /// never existed or because it has since fallen out of every source's retained
    /// window.
    async fn snapshot_for(&self, height: Height) -> Result<TrustedSnapshot, DirectoryClientError> {
        if let Some(snapshot) = self.state.lock().await.snapshots.get(&height) {
            return Ok(snapshot.clone());
        }

        let candidates = join_all(self.sources.iter().map(|s| s.snapshot_at(height)))
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect();

        let (agreed_height, trusted) = match self.reach_quorum(candidates) {
            Ok(agreed) => agreed,
            Err(DirectoryClientError::QuorumNotReached { .. }) => {
                return Err(DirectoryClientError::NoQuorumSnapshotForHeight(
                    height.value(),
                ));
            }
            Err(other) => return Err(other),
        };
        if agreed_height != height {
            return Err(DirectoryClientError::NoQuorumSnapshotForHeight(
                height.value(),
            ));
        }

        self.state
            .lock()
            .await
            .snapshots
            .insert(height, trusted.clone());
        Ok(trusted)
    }
}

/// Append `bytes` prefixed with its u32 little-endian length, so adjacent
/// variable-length fields cannot be confused with one another. Mirrors
/// `nym_directory_contract_common::helpers::push_len_prefixed`'s framing (private to
/// that crate); reproduced here since it is the only encoder in this crate that needs it.
fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// The exact bytes a nym-api signs when attesting a directory snapshot: the block
/// `app_hash`, the directory's LtHash `accumulator`, and a hash over the current
/// `NodeId -> ed25519 identity` mapping (see
/// [`crate::verify::node_identities_hash`]), all bound to a chain-id, contract address,
/// and height so a signature cannot be replayed across chains, contract instances, or
/// heights.
pub(crate) fn digest_snapshot_signing_payload(
    chain_id: &str,
    contract: &AccountId,
    height: Height,
    app_hash: &AppHash,
    accumulator: &LtHash16,
    node_identities_hash: &[u8; 32],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(DIGEST_SNAPSHOT_DOMAIN_TAG);
    push_len_prefixed(&mut buf, chain_id.as_bytes());
    push_len_prefixed(&mut buf, &contract.to_bytes());
    buf.extend_from_slice(&height.value().to_le_bytes());
    push_len_prefixed(&mut buf, app_hash.as_bytes());
    push_len_prefixed(&mut buf, &accumulator.to_bytes());
    buf.extend_from_slice(node_identities_hash);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::ed25519::{KeyPair, PublicKey};
    use nym_test_utils::helpers::u64_seeded_rng;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn contract() -> AccountId {
        AccountId::from_str("n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr").unwrap()
    }

    fn other_contract() -> AccountId {
        AccountId::from_str("n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf").unwrap()
    }

    fn app_hash(byte: u8) -> AppHash {
        AppHash::try_from(vec![byte; 32]).unwrap()
    }

    fn keypair(seed: u64) -> KeyPair {
        let mut rng = u64_seeded_rng(seed);
        KeyPair::new(&mut rng)
    }

    fn signed_snapshot_with(
        kp: &KeyPair,
        chain_id: &str,
        contract: &AccountId,
        height: Height,
        app_hash: AppHash,
        accumulator: LtHash16,
        node_identities_hash: [u8; 32],
    ) -> SignedDigestSnapshot {
        let snapshot = DigestSnapshot {
            chain_id: chain::Id::try_from(chain_id).unwrap(),
            directory_contract: contract.clone(),
            height,
            app_hash,
            accumulator,
            node_identities_hash,
        };
        let signature = kp.private_key().sign(snapshot.signing_payload());
        SignedDigestSnapshot {
            snapshot,
            signer: *kp.public_key(),
            signature,
        }
    }

    fn signed_snapshot(
        kp: &KeyPair,
        chain_id: &str,
        contract: &AccountId,
        height: Height,
    ) -> SignedDigestSnapshot {
        signed_snapshot_with(
            kp,
            chain_id,
            contract,
            height,
            app_hash(1),
            LtHash16::new(),
            [0u8; 32],
        )
    }

    #[test]
    fn digest_snapshot_payload_is_deterministic_and_field_sensitive() {
        let contract = contract();
        let acc = LtHash16::new();
        let node_hash = [9u8; 32];
        let base = digest_snapshot_signing_payload(
            "nyx-testnet",
            &contract,
            Height::from(100u32),
            &app_hash(1),
            &acc,
            &node_hash,
        );
        assert_eq!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-mainnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &other_contract(),
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(101u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(2),
                &acc,
                &node_hash,
            )
        );
        let mut other_acc = LtHash16::new();
        other_acc.add(b"leaf");
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &other_acc,
                &node_hash,
            )
        );
        let mut other_node_hash = node_hash;
        other_node_hash[0] ^= 1;
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &other_node_hash,
            )
        );
    }

    #[test]
    fn digest_snapshot_payload_length_prefix_disambiguates() {
        // (chain-id "ab", contract-derived bytes) framing must not let adjacent
        // variable-length fields bleed into one another; exercised here via chain-id
        // vs. the contract's encoded bytes rather than two strings of our own choosing,
        // since `contract` is a real bech32 address.
        let acc = LtHash16::new();
        let node_hash = [0u8; 32];
        assert_ne!(
            digest_snapshot_signing_payload(
                "ab",
                &contract(),
                Height::from(0u32),
                &app_hash(0),
                &acc,
                &node_hash,
            ),
            digest_snapshot_signing_payload(
                "a",
                &other_contract(),
                Height::from(0u32),
                &app_hash(0),
                &acc,
                &node_hash,
            ),
        );
    }

    #[test]
    fn digest_snapshot_payload_is_domain_tagged() {
        let payload = digest_snapshot_signing_payload(
            "chain",
            &contract(),
            Height::from(1u32),
            &app_hash(7),
            &LtHash16::new(),
            &[7u8; 32],
        );
        assert!(payload.starts_with(DIGEST_SNAPSHOT_DOMAIN_TAG));

        // a representative node-entry payload never starts with the snapshot's domain
        // tag, so the two signature domains cannot be confused
        let node_payload = nym_directory_contract_common::node_signing_payload(1, "x", 1, b"y");
        assert!(!node_payload.starts_with(DIGEST_SNAPSHOT_DOMAIN_TAG));
    }

    #[test]
    fn verify_accepts_a_valid_attestation_from_a_trusted_signer() {
        let kp = keypair(1);
        let trusted = HashSet::from([*kp.public_key()]);
        let snapshot = signed_snapshot(&kp, "nyx-testnet", &contract(), Height::from(100u32));

        assert!(snapshot.verify(&trusted, &"nyx-testnet".parse().unwrap(), &contract()));
    }

    #[test]
    fn verify_rejects_an_untrusted_signer() {
        let kp = keypair(1);
        let other = keypair(2);
        let trusted = HashSet::from([*other.public_key()]);
        let snapshot = signed_snapshot(&kp, "nyx-testnet", &contract(), Height::from(100u32));

        assert!(!snapshot.verify(&trusted, &"nyx-testnet".parse().unwrap(), &contract()));
    }

    #[test]
    fn verify_rejects_a_mismatched_chain_id_or_contract() {
        let kp = keypair(1);
        let trusted = HashSet::from([*kp.public_key()]);
        let snapshot = signed_snapshot(&kp, "nyx-testnet", &contract(), Height::from(100u32));

        assert!(!snapshot.verify(&trusted, &"nyx-mainnet".parse().unwrap(), &contract()));
        assert!(!snapshot.verify(&trusted, &"nyx-testnet".parse().unwrap(), &other_contract()));
    }

    #[test]
    fn verify_rejects_a_forged_or_malformed_signature() {
        let kp = keypair(1);
        let trusted = HashSet::from([*kp.public_key()]);

        let mut forged = signed_snapshot(&kp, "nyx-testnet", &contract(), Height::from(100u32));
        forged.signature = keypair(2)
            .private_key()
            .sign(forged.snapshot.signing_payload());
        assert!(!forged.verify(&trusted, &"nyx-testnet".parse().unwrap(), &contract()));

        let mut malformed = signed_snapshot(&kp, "nyx-testnet", &contract(), Height::from(101u32));
        malformed.signature = forged.signature;
        assert!(!malformed.verify(&trusted, &"nyx-testnet".parse().unwrap(), &contract()));
    }

    fn anchor(trusted: HashSet<ed25519::PublicKey>, quorum: usize) -> AttestedTrustAnchor<()> {
        AttestedTrustAnchor::new(
            Vec::new(),
            trusted,
            quorum,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap()
    }

    #[test]
    fn new_rejects_zero_quorum() {
        let trusted = HashSet::from([*keypair(1).public_key()]);
        let result = AttestedTrustAnchor::<()>::new(
            Vec::new(),
            trusted,
            0,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        );
        assert!(matches!(
            result,
            Err(DirectoryClientError::InvalidQuorumConfig {
                quorum: 0,
                signers: 1
            })
        ));
    }

    #[test]
    fn new_rejects_quorum_exceeding_signer_count() {
        let trusted = HashSet::from([*keypair(1).public_key()]);
        let result = AttestedTrustAnchor::<()>::new(
            Vec::new(),
            trusted,
            2,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        );
        assert!(matches!(
            result,
            Err(DirectoryClientError::InvalidQuorumConfig {
                quorum: 2,
                signers: 1
            })
        ));
    }

    #[test]
    fn new_accepts_a_valid_configuration() {
        let trusted = HashSet::from([*keypair(1).public_key(), *keypair(2).public_key()]);
        assert!(
            AttestedTrustAnchor::<()>::new(
                Vec::new(),
                trusted,
                2,
                chain::Id::try_from("nyx-testnet").unwrap(),
                contract(),
            )
            .is_ok()
        );
    }

    #[test]
    fn majority_quorum_is_more_than_half() {
        assert_eq!(AttestedTrustAnchor::<()>::majority_quorum(1), 1);
        assert_eq!(AttestedTrustAnchor::<()>::majority_quorum(2), 2);
        assert_eq!(AttestedTrustAnchor::<()>::majority_quorum(3), 2);
        assert_eq!(AttestedTrustAnchor::<()>::majority_quorum(4), 3);
        assert_eq!(AttestedTrustAnchor::<()>::majority_quorum(5), 3);
    }

    #[test]
    fn with_default_anchor_uses_the_compiled_in_default() {
        let anchor = AttestedTrustAnchor::<()>::with_default_anchor(
            Vec::new(),
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap();

        assert_eq!(anchor.trusted_signers, default_trusted_signers());
        assert_eq!(
            anchor.quorum,
            AttestedTrustAnchor::<()>::majority_quorum(anchor.trusted_signers.len())
        );
    }

    #[test]
    fn new_with_a_caller_supplied_set_is_unaffected_by_the_default() {
        let custom = HashSet::from([*keypair(1).public_key()]);
        let anchor = AttestedTrustAnchor::<()>::new(
            Vec::new(),
            custom.clone(),
            1,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap();

        assert_eq!(anchor.trusted_signers, custom);
        assert_ne!(anchor.trusted_signers, default_trusted_signers());
    }

    #[test]
    fn reach_quorum_accepts_k_distinct_agreeing_signers() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = anchor(trusted, 2);

        let candidates = vec![
            signed_snapshot(&a, "nyx-testnet", &contract(), Height::from(100u32)),
            signed_snapshot(&b, "nyx-testnet", &contract(), Height::from(100u32)),
        ];

        let (height, _) = anchor.reach_quorum(candidates).unwrap();
        assert_eq!(height, Height::from(100u32));
    }

    #[test]
    fn reach_quorum_fails_with_fewer_than_k_agreeing_signers() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = anchor(trusted, 2);

        let candidates = vec![signed_snapshot(
            &a,
            "nyx-testnet",
            &contract(),
            Height::from(100u32),
        )];

        let err = anchor.reach_quorum(candidates).unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[test]
    fn reach_quorum_counts_a_duplicated_signer_once() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = anchor(trusted, 2);

        // the same signer's attestation presented twice must not, by itself, reach a
        // quorum of 2
        let candidates = vec![
            signed_snapshot(&a, "nyx-testnet", &contract(), Height::from(100u32)),
            signed_snapshot(&a, "nyx-testnet", &contract(), Height::from(100u32)),
        ];

        let err = anchor.reach_quorum(candidates).unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[test]
    fn reach_quorum_ignores_untrusted_or_invalid_attestations() {
        let a = keypair(1);
        let untrusted = keypair(2);
        let trusted = HashSet::from([*a.public_key()]);
        let anchor = anchor(trusted, 1);

        let mut forged = signed_snapshot(&a, "nyx-testnet", &contract(), Height::from(100u32));
        forged.signature = untrusted
            .private_key()
            .sign(forged.snapshot.signing_payload());

        let candidates = vec![
            signed_snapshot(&untrusted, "nyx-testnet", &contract(), Height::from(100u32)),
            forged,
        ];

        let err = anchor.reach_quorum(candidates).unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 1,
                agreed: 0
            }
        ));
    }

    #[test]
    fn reach_quorum_rejects_disagreeing_signers() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = anchor(trusted, 2);

        // a and b sign DIFFERENT accumulators at the same height - no single value
        // reaches the quorum of 2
        let candidates = vec![
            signed_snapshot_with(
                &a,
                "nyx-testnet",
                &contract(),
                Height::from(100u32),
                app_hash(1),
                LtHash16::new(),
                [0u8; 32],
            ),
            signed_snapshot_with(
                &b,
                "nyx-testnet",
                &contract(),
                Height::from(100u32),
                app_hash(2),
                LtHash16::new(),
                [0u8; 32],
            ),
        ];

        let err = anchor.reach_quorum(candidates).unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    struct MockSource {
        identity: ed25519::PublicKey,
        latest: Option<SignedDigestSnapshot>,
        by_height: HashMap<Height, SignedDigestSnapshot>,
    }

    #[async_trait]
    impl AttestationSource for MockSource {
        fn identity(&self) -> PublicKey {
            self.identity
        }

        async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, DirectoryClientError> {
            self.latest
                .clone()
                .ok_or(DirectoryClientError::NoQuorumSnapshotForHeight(0))
        }

        async fn snapshot_at(
            &self,
            height: Height,
        ) -> Result<SignedDigestSnapshot, DirectoryClientError> {
            self.by_height.get(&height).cloned().ok_or(
                DirectoryClientError::NoQuorumSnapshotForHeight(height.value()),
            )
        }
    }

    fn mock_source(kp: &KeyPair, height: Height) -> MockSource {
        let snapshot = signed_snapshot(kp, "nyx-testnet", &contract(), height);
        MockSource {
            identity: *kp.public_key(),
            latest: Some(snapshot.clone()),
            by_height: HashMap::from([(height, snapshot)]),
        }
    }

    #[tokio::test]
    async fn refresh_seeds_a_height_and_confirms_it_across_sources() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            mock_source(&a, Height::from(100u32)),
            mock_source(&b, Height::from(100u32)),
        ];
        let anchor = AttestedTrustAnchor::new(
            sources,
            trusted,
            2,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap();

        let height = anchor.refresh().await.unwrap();
        assert_eq!(height, Height::from(100u32));
        assert_eq!(anchor.latest_snapshot_height().await.unwrap(), height);
    }

    #[tokio::test]
    async fn refresh_fails_when_sources_disagree() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        // a and b each only ever answer for their OWN, different height, so asking the
        // other for the seeded height always comes back empty
        let sources = vec![
            mock_source(&a, Height::from(100u32)),
            mock_source(&b, Height::from(200u32)),
        ];
        let anchor = AttestedTrustAnchor::new(
            sources,
            trusted,
            2,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap();

        let err = anchor.refresh().await.unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[tokio::test]
    async fn snapshot_for_returns_the_cached_value_on_a_second_call() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            mock_source(&a, Height::from(100u32)),
            mock_source(&b, Height::from(100u32)),
        ];
        let anchor = AttestedTrustAnchor::new(
            sources,
            trusted,
            2,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap();

        let first = anchor.snapshot_for(Height::from(100u32)).await.unwrap();
        let second = anchor.snapshot_for(Height::from(100u32)).await.unwrap();
        assert_eq!(first.accumulator, second.accumulator);
    }

    #[tokio::test]
    async fn snapshot_for_rejects_a_height_no_quorum_can_attest() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            mock_source(&a, Height::from(100u32)),
            mock_source(&b, Height::from(100u32)),
        ];
        let anchor = AttestedTrustAnchor::new(
            sources,
            trusted,
            2,
            chain::Id::try_from("nyx-testnet").unwrap(),
            contract(),
        )
        .unwrap();

        let err = anchor.snapshot_for(Height::from(999u32)).await.unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::NoQuorumSnapshotForHeight(999)
        ));
    }
}
