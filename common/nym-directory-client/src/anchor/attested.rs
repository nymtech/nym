// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::{DirectoryTrustAnchor, TrustedDigest};
use crate::error::DirectoryClientError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::chain;
use futures::future::join_all;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{AttestationSource, DigestSnapshot, SignedDigestSnapshot};
use nym_lthash::LtHash16;
use nym_network_defaults::default_directory_attestation_sources;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::hash::AppHash;
use rand::seq::SliceRandom;
use std::collections::{BTreeMap, HashMap, HashSet};
use tokio::sync::Mutex;

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

    /// The trusted hash over the `NodeId -> ed25519 identity` mapping at `height` (see
    /// [`crate::verify::node_identities_hash`]) - anchor-specific rather than part of
    /// the shared [`DirectoryTrustAnchor`] trait, since `ProvenTrustAnchor` and
    /// `LightClientAnchor` have no equivalent value to offer.
    pub async fn trusted_node_identities_hash(
        &self,
        height: Height,
    ) -> Result<[u8; 32], DirectoryClientError> {
        Ok(self.snapshot_for(height).await?.node_identities_hash)
    }
}

#[async_trait]
impl<S> DirectoryTrustAnchor for AttestedTrustAnchor<S>
where
    S: AttestationSource + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, DirectoryClientError> {
        Ok(self.snapshot_for(height).await?.app_hash)
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError> {
        let snapshot = self.snapshot_for(height).await?;
        Ok(TrustedDigest {
            height,
            accumulator: snapshot.accumulator,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::keypair;
    use nym_directory_attestation::source::mock::{
        mock_app_hash, mock_attestation_source, mock_chain_id, mock_contract, mock_digest_snapshot,
    };

    fn mock_anchor(trusted: HashSet<ed25519::PublicKey>, quorum: usize) -> AttestedTrustAnchor<()> {
        AttestedTrustAnchor::new(
            Vec::new(),
            trusted,
            quorum,
            mock_chain_id(),
            mock_contract(0),
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
            mock_chain_id(),
            mock_contract(0),
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
            mock_chain_id(),
            mock_contract(0),
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
                mock_chain_id(),
                mock_contract(0),
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
            mock_chain_id(),
            mock_contract(0),
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
            mock_chain_id(),
            mock_contract(0),
        )
        .unwrap();

        assert_eq!(anchor.trusted_signers, custom);
        assert_ne!(anchor.trusted_signers, default_trusted_signers());
    }

    #[test]
    fn reach_quorum_accepts_k_distinct_agreeing_signers() {
        let a = keypair(1);
        let b = keypair(2);
        let height = Height::from(100u32);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = mock_anchor(trusted, 2);

        let candidates = vec![
            mock_digest_snapshot(height).signed(&a),
            mock_digest_snapshot(height).signed(&b),
        ];

        let (height, _) = anchor.reach_quorum(candidates).unwrap();
        assert_eq!(height, Height::from(100u32));
    }

    #[test]
    fn reach_quorum_fails_with_fewer_than_k_agreeing_signers() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = mock_anchor(trusted, 2);

        let candidates = vec![mock_digest_snapshot(Height::from(100u32)).signed(&a)];

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
        let height = Height::from(100u32);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = mock_anchor(trusted, 2);

        // the same signer's attestation presented twice must not, by itself, reach a
        // quorum of 2
        let candidates = vec![
            mock_digest_snapshot(height).signed(&a),
            mock_digest_snapshot(height).signed(&a),
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
        let height = Height::from(100u32);
        let trusted = HashSet::from([*a.public_key()]);
        let anchor = mock_anchor(trusted, 1);

        let mut forged = mock_digest_snapshot(height).signed(&a);
        forged.signature = untrusted
            .private_key()
            .sign(forged.snapshot.signing_payload());

        let candidates = vec![mock_digest_snapshot(height).signed(&untrusted), forged];

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
        let height = Height::from(100u32);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = mock_anchor(trusted, 2);

        // a and b sign DIFFERENT accumulators at the same height - no single value
        // reaches the quorum of 2
        let mut snapshot1 = mock_digest_snapshot(height);
        snapshot1.app_hash = mock_app_hash(1);
        let mut snapshot2 = mock_digest_snapshot(height);
        snapshot2.app_hash = mock_app_hash(2);

        let candidates = vec![snapshot1.signed(&a), snapshot2.signed(&b)];

        let err = anchor.reach_quorum(candidates).unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[tokio::test]
    async fn refresh_seeds_a_height_and_confirms_it_across_sources() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            mock_attestation_source(&a, Height::from(100u32)),
            mock_attestation_source(&b, Height::from(100u32)),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let height = anchor.refresh().await.unwrap();
        assert_eq!(height, Height::from(100u32));
        assert_eq!(anchor.latest_snapshot_height().await.unwrap(), height);
    }

    #[tokio::test]
    async fn refresh_pins_the_height_and_a_cached_query_does_not_requery_sources() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let source_a = mock_attestation_source(&a, Height::from(100u32));
        let source_b = mock_attestation_source(&b, Height::from(100u32));
        let sources = vec![source_a.clone(), source_b.clone()];

        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let height = anchor.refresh().await.unwrap();
        assert_eq!(height, Height::from(100u32));

        // exactly one source was asked for "latest" (the randomly-chosen seed); the
        // seed itself is never re-queried for confirmation (excluded via `identity()`,
        // see design.md D6), so exactly one `snapshot_at` call happened in total too
        let latest_calls_after_refresh =
            source_a.latest_snapshot_calls() + source_b.latest_snapshot_calls();
        let snapshot_at_calls_after_refresh =
            source_a.snapshot_at_calls().len() + source_b.snapshot_at_calls().len();
        assert_eq!(latest_calls_after_refresh, 1);
        assert_eq!(snapshot_at_calls_after_refresh, 1);

        // a later query for the now-cached height is served from cache - no source is
        // queried again at all
        assert!(anchor.trusted_app_hash(height).await.is_ok());
        assert_eq!(
            source_a.latest_snapshot_calls() + source_b.latest_snapshot_calls(),
            latest_calls_after_refresh
        );
        assert_eq!(
            source_a.snapshot_at_calls().len() + source_b.snapshot_at_calls().len(),
            snapshot_at_calls_after_refresh
        );
    }

    #[tokio::test]
    async fn refresh_fails_when_sources_disagree() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        // a and b each only ever answer for their OWN, different height, so asking the
        // other for the seeded height always comes back empty
        let sources = vec![
            mock_attestation_source(&a, Height::from(100u32)),
            mock_attestation_source(&b, Height::from(200u32)),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
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
            mock_attestation_source(&a, Height::from(100u32)),
            mock_attestation_source(&b, Height::from(100u32)),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
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
            mock_attestation_source(&a, Height::from(100u32)),
            mock_attestation_source(&b, Height::from(100u32)),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let err = anchor.snapshot_for(Height::from(999u32)).await.unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::NoQuorumSnapshotForHeight(999)
        ));
    }

    #[tokio::test]
    async fn directory_trust_anchor_impl_returns_the_attested_values() {
        let a = keypair(1);
        let b = keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let height = Height::from(100u32);

        // the anchor must surface exactly what the quorum's mock snapshot committed
        let expected = mock_digest_snapshot(height);
        let sources = [&a, &b].map(|kp| mock_attestation_source(kp, height));

        let anchor = AttestedTrustAnchor::new(
            sources.into(),
            trusted,
            2,
            mock_chain_id(),
            mock_contract(0),
        )
        .unwrap();

        assert_eq!(
            anchor.trusted_app_hash(height).await.unwrap(),
            expected.app_hash
        );

        let digest = anchor.trusted_digest(height).await.unwrap();
        assert_eq!(digest.height, height);
        assert_eq!(digest.accumulator, expected.accumulator);

        assert_eq!(
            anchor.trusted_node_identities_hash(height).await.unwrap(),
            expected.node_identities_hash
        );
    }
}
