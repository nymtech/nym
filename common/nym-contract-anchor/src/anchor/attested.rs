// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::{TrustAnchor, TrustedDigest};
use crate::error::AnchorError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::chain;
use futures::future::join_all;
use nym_contract_attestation::{AttestationSource, DigestSnapshot, SignedDigestSnapshot};
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use nym_network_defaults::default_contract_attestation_sources;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::hash::AppHash;
use std::collections::{BTreeMap, HashMap, HashSet};
use tokio::sync::Mutex;

/// A quorum-agreed `app_hash`, digest accumulator, and node-identity hash for a
/// specific height - the trusted output of the anchor's quorum.
///
/// Public so a domain client can verify its own record set against these values without
/// the anchor knowing that domain's types. See [`AttestedTrustAnchor::trusted_snapshot`].
#[derive(Clone, Debug)]
pub struct TrustedSnapshot {
    pub app_hash: AppHash,
    pub accumulator: LtHash16,
    pub node_identities_hash: [u8; 32],
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
    default_contract_attestation_sources()
        .iter()
        .map(|source| {
            ed25519::PublicKey::from_base58_string(&source.identity_ed25519_bs58)
                .expect("compiled-in default trusted signer key must be valid")
        })
        .collect()
}

/// A [`TrustAnchor`] backed by a K-of-N
/// quorum of nym-api identity keys signing directory snapshots, rather than a root key
/// or a light-client checkpoint.
pub struct AttestedTrustAnchor<S> {
    sources: Vec<S>,
    trusted_signers: HashSet<ed25519::PublicKey>,
    quorum: usize,
    chain_id: chain::Id,
    contract: AccountId,

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
        contract: AccountId,
    ) -> Result<Self, AnchorError> {
        if quorum == 0 || quorum > trusted_signers.len() {
            return Err(AnchorError::InvalidQuorumConfig {
                quorum,
                signers: trusted_signers.len(),
            });
        }

        Ok(Self {
            sources,
            trusted_signers,
            quorum,
            chain_id,
            contract,
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
    /// [`nym_network_defaults::mainnet::CONTRACT_ATTESTATION_SOURCES`]' identity keys,
    /// requiring [`Self::majority_quorum`] of them to agree. This is the common case,
    /// since most deployments have no reason to distrust Nym SA's own instances;
    /// callers who do, or who are not on mainnet, should use [`Self::new`] directly.
    pub fn with_default_anchor(
        sources: Vec<S>,
        chain_id: chain::Id,
        contract: AccountId,
    ) -> Result<Self, AnchorError> {
        let trusted_signers = default_trusted_signers();
        let quorum = Self::majority_quorum(trusted_signers.len());
        Self::new(sources, trusted_signers, quorum, chain_id, contract)
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
    ) -> Result<(Height, TrustedSnapshot), AnchorError> {
        // map between returned snapshot and signers which attested it
        let mut groups: HashMap<DigestSnapshot, HashSet<ed25519::PublicKey>> = HashMap::new();

        for candidate in candidates {
            // disregard any inconsistent responses
            if !candidate.verify(&self.trusted_signers, &self.chain_id, &self.contract) {
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

        Err(AnchorError::QuorumNotReached {
            needed: self.quorum,
            agreed: best_agreed,
        })
    }
}

impl<S> AttestedTrustAnchor<S>
where
    S: AttestationSource + Sync,
{
    /// Queries every source's [`AttestationSource::latest_snapshot`] concurrently and keeps
    /// the attestations that verify on their own (trusted signer, correct chain + contract
    /// scope, valid signature) - the candidate pool [`Self::refresh`] confirms heights from.
    async fn latest_snapshot_candidates(&self) -> Vec<SignedDigestSnapshot> {
        join_all(self.sources.iter().map(|s| s.latest_snapshot()))
            .await
            .into_iter()
            .filter_map(Result::ok)
            .filter(|candidate| {
                candidate.verify(&self.trusted_signers, &self.chain_id, &self.contract)
            })
            .collect()
    }

    /// Discovers and pins the quorum-agreed latest snapshot: collects every source's own
    /// (individually verified) "latest" attestation, then tries the claimed heights
    /// highest-first - reusing the latest attestations already at that height and asking
    /// every remaining source's [`AttestationSource::snapshot_at`] - until one height
    /// reaches quorum. A source claiming a stale height therefore cannot steer the pin
    /// (the highest quorum-confirmable claim wins), and a source claiming an
    /// unconfirmable height merely falls through to the next height down. The pin also
    /// never moves backwards: a quorum'd height below the current latest is cached for
    /// explicit per-height queries, but `latest` (and the returned height) stays.
    pub async fn refresh(&self) -> Result<Height, AnchorError> {
        let latest_candidates = self.latest_snapshot_candidates().await;

        // the distinct claimed heights, tried highest-first below
        let mut heights: Vec<Height> = latest_candidates
            .iter()
            .map(|c| c.snapshot.height)
            .collect();
        heights.sort_unstable();
        heights.dedup();

        let mut best_agreed = 0;
        let mut confirmed = None;
        for height in heights.into_iter().rev() {
            let mut candidates: Vec<_> = latest_candidates
                .iter()
                .filter(|c| c.snapshot.height == height)
                .cloned()
                .collect();
            let already_answered: HashSet<ed25519::PublicKey> =
                candidates.iter().map(|c| c.signer).collect();
            candidates.extend(
                join_all(
                    self.sources
                        .iter()
                        .filter(|s| !already_answered.contains(&s.identity()))
                        .map(|s| s.snapshot_at(height)),
                )
                .await
                .into_iter()
                .filter_map(Result::ok),
            );

            match self.reach_quorum(candidates) {
                Ok(quorum) => {
                    confirmed = Some(quorum);
                    break;
                }
                Err(AnchorError::QuorumNotReached { agreed, .. }) => {
                    best_agreed = best_agreed.max(agreed);
                }
                Err(other) => return Err(other),
            }
        }

        let Some((height, trusted)) = confirmed else {
            return Err(AnchorError::QuorumNotReached {
                needed: self.quorum,
                agreed: best_agreed,
            });
        };

        let mut state = self.state.lock().await;
        state.snapshots.insert(height, trusted);
        match state.latest {
            Some(existing) if existing > height => Ok(existing),
            _ => {
                state.latest = Some(height);
                Ok(height)
            }
        }
    }

    /// The cached latest quorum-agreed height, or [`Self::refresh`] if none is cached
    /// yet.
    pub async fn latest_snapshot_height(&self) -> Result<Height, AnchorError> {
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
    /// [`AnchorError::NoQuorumSnapshotForHeight`], whether that is because it
    /// never existed or because it has since fallen out of every source's retained
    /// window.
    async fn snapshot_for(&self, height: Height) -> Result<TrustedSnapshot, AnchorError> {
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
            Err(AnchorError::QuorumNotReached { .. }) => {
                return Err(AnchorError::NoQuorumSnapshotForHeight(height.value()));
            }
            Err(other) => return Err(other),
        };
        if agreed_height != height {
            return Err(AnchorError::NoQuorumSnapshotForHeight(height.value()));
        }

        self.state
            .lock()
            .await
            .snapshots
            .insert(height, trusted.clone());
        Ok(trusted)
    }

    /// The trusted hash over the `NodeId -> ed25519 identity` mapping at `height` (see
    /// [`nym_contract_attestation::node_identities_hash`]) - anchor-specific rather than
    /// part of the shared [`TrustAnchor`] trait, since `ProvenTrustAnchor` and
    /// `LightClientAnchor` have no equivalent value to offer.
    pub async fn trusted_node_identities_hash(
        &self,
        height: Height,
    ) -> Result<[u8; 32], AnchorError> {
        Ok(self.snapshot_for(height).await?.node_identities_hash)
    }

    /// The quorum-agreed values at `height`: the `app_hash`, the accumulator, and the
    /// node-identities hash, reusing the quorum and the cache.
    ///
    /// The seam a domain client verifies its own record set through - the anchor establishes
    /// what is trusted, the client knows what its records mean.
    pub async fn trusted_snapshot(&self, height: Height) -> Result<TrustedSnapshot, AnchorError> {
        self.snapshot_for(height).await
    }

    /// The configured attestation sources, so a domain client can fetch the bulk data this
    /// anchor has attested the hashes of.
    pub fn sources(&self) -> &[S] {
        &self.sources
    }
}

#[async_trait]
impl<S> TrustAnchor for AttestedTrustAnchor<S>
where
    S: AttestationSource + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
        Ok(self.snapshot_for(height).await?.app_hash)
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
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
    use nym_contract_attestation::source::mock::{
        MockAttestationSource, mock_app_hash, mock_attestation_source, mock_chain_id,
        mock_contract, mock_digest_snapshot,
    };
    use nym_test_utils::helpers::dummy_ed25519_keypair;

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
        let trusted = HashSet::from([*dummy_ed25519_keypair(1).public_key()]);
        let result = AttestedTrustAnchor::<()>::new(
            Vec::new(),
            trusted,
            0,
            mock_chain_id(),
            mock_contract(0),
        );
        assert!(matches!(
            result,
            Err(AnchorError::InvalidQuorumConfig {
                quorum: 0,
                signers: 1
            })
        ));
    }

    #[test]
    fn new_rejects_quorum_exceeding_signer_count() {
        let trusted = HashSet::from([*dummy_ed25519_keypair(1).public_key()]);
        let result = AttestedTrustAnchor::<()>::new(
            Vec::new(),
            trusted,
            2,
            mock_chain_id(),
            mock_contract(0),
        );
        assert!(matches!(
            result,
            Err(AnchorError::InvalidQuorumConfig {
                quorum: 2,
                signers: 1
            })
        ));
    }

    #[test]
    fn new_accepts_a_valid_configuration() {
        let trusted = HashSet::from([
            *dummy_ed25519_keypair(1).public_key(),
            *dummy_ed25519_keypair(2).public_key(),
        ]);
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
        let custom = HashSet::from([*dummy_ed25519_keypair(1).public_key()]);
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
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let anchor = mock_anchor(trusted, 2);

        let candidates = vec![mock_digest_snapshot(Height::from(100u32)).signed(&a)];

        let err = anchor.reach_quorum(candidates).unwrap_err();
        assert!(matches!(
            err,
            AnchorError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[test]
    fn reach_quorum_counts_a_duplicated_signer_once() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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
            AnchorError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[test]
    fn reach_quorum_ignores_untrusted_or_invalid_attestations() {
        let a = dummy_ed25519_keypair(1);
        let untrusted = dummy_ed25519_keypair(2);
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
            AnchorError::QuorumNotReached {
                needed: 1,
                agreed: 0
            }
        ));
    }

    #[test]
    fn reach_quorum_rejects_disagreeing_signers() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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
            AnchorError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[tokio::test]
    async fn refresh_seeds_a_height_and_confirms_it_across_sources() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let source_a = mock_attestation_source(&a, Height::from(100u32));
        let source_b = mock_attestation_source(&b, Height::from(100u32));
        let sources = vec![source_a.clone(), source_b.clone()];

        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let height = anchor.refresh().await.unwrap();
        assert_eq!(height, Height::from(100u32));

        // every source was asked for its "latest" exactly once; both of those attestations
        // already covered the confirmed height, so no snapshot_at confirmation was needed
        let latest_calls_after_refresh =
            source_a.latest_snapshot_calls() + source_b.latest_snapshot_calls();
        let snapshot_at_calls_after_refresh =
            source_a.snapshot_at_calls().len() + source_b.snapshot_at_calls().len();
        assert_eq!(latest_calls_after_refresh, 2);
        assert_eq!(snapshot_at_calls_after_refresh, 0);

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

    // a source with the given "latest" claim that can also answer snapshot_at for `known`
    fn source_with_heights(
        kp: &ed25519::KeyPair,
        latest: Height,
        known: &[Height],
    ) -> MockAttestationSource {
        let by_height = known
            .iter()
            .map(|&h| (h, mock_digest_snapshot(h).signed(kp)))
            .collect();
        MockAttestationSource::new(
            *kp.public_key(),
            mock_digest_snapshot(latest).signed(kp),
            by_height,
        )
    }

    #[tokio::test]
    async fn refresh_pins_the_highest_quorum_confirmable_height() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let h100 = Height::from(100u32);
        let h200 = Height::from(200u32);

        // b's own "latest" lags at 100, but it can still confirm 200 on request - a
        // lagging source must not drag the pin down to its stale claim
        let sources = vec![
            source_with_heights(&a, h200, &[h100, h200]),
            source_with_heights(&b, h100, &[h100, h200]),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        assert_eq!(anchor.refresh().await.unwrap(), h200);
    }

    #[tokio::test]
    async fn refresh_falls_back_when_the_highest_claim_cannot_be_confirmed() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let h100 = Height::from(100u32);
        let h200 = Height::from(200u32);

        // a claims 200 but no one else can confirm it; the quorum forms at 100 instead of
        // the whole refresh failing
        let sources = vec![
            source_with_heights(&a, h200, &[h100, h200]),
            source_with_heights(&b, h100, &[h100]),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        assert_eq!(anchor.refresh().await.unwrap(), h100);
    }

    #[tokio::test]
    async fn refresh_never_regresses_the_latest_pin() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let h100 = Height::from(100u32);
        let h200 = Height::from(200u32);

        // every source now only serves height 100 (e.g. replayed stale attestations)
        let sources = vec![
            mock_attestation_source(&a, h100),
            mock_attestation_source(&b, h100),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        // a previous refresh already pinned 200
        {
            let mut state = anchor.state.lock().await;
            state.latest = Some(h200);
            state.snapshots.insert(
                h200,
                TrustedSnapshot::from_snapshot(mock_digest_snapshot(h200)),
            );
        }

        // the quorum'd-but-older height must not roll the pin back
        assert_eq!(anchor.refresh().await.unwrap(), h200);

        let state = anchor.state.lock().await;
        assert_eq!(state.latest, Some(h200));
        // the fresh quorum'd height is still cached for explicit per-height queries
        assert!(state.snapshots.contains_key(&h100));
    }

    #[tokio::test]
    async fn refresh_fails_when_sources_disagree() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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
            AnchorError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[tokio::test]
    async fn snapshot_for_returns_the_cached_value_on_a_second_call() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            mock_attestation_source(&a, Height::from(100u32)),
            mock_attestation_source(&b, Height::from(100u32)),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let err = anchor.snapshot_for(Height::from(999u32)).await.unwrap_err();
        assert!(matches!(err, AnchorError::NoQuorumSnapshotForHeight(999)));
    }

    #[tokio::test]
    async fn directory_trust_anchor_impl_returns_the_attested_values() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
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

    // --- one anchor type, many contracts ---

    /// A snapshot naming `contract`, carrying `accumulator`, signed by `kp`, served as that
    /// signer's latest and at `height`.
    fn source_for_contract(
        kp: &ed25519::KeyPair,
        contract: &AccountId,
        height: Height,
        accumulator: LtHash16,
    ) -> MockAttestationSource {
        let snapshot = DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: contract.clone(),
            height,
            app_hash: mock_app_hash(1),
            accumulator,
            node_identities_hash: [0u8; 32],
        }
        .signed(kp);
        MockAttestationSource::new(
            *kp.public_key(),
            snapshot.clone(),
            HashMap::from([(height, snapshot)]),
        )
    }

    fn accumulator_over(leaf: &[u8]) -> LtHash16 {
        let mut acc = LtHash16::new();
        acc.add(leaf);
        acc
    }

    #[tokio::test]
    async fn one_anchor_type_serves_two_contracts() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let height = Height::from(100u32);

        // two different contracts at the same height, each committing its own accumulator
        let first = mock_contract(0);
        let second = mock_contract(1);
        let first_acc = accumulator_over(b"first-contract-leaf");
        let second_acc = accumulator_over(b"second-contract-leaf");
        assert_ne!(first_acc, second_acc);

        let first_anchor = AttestedTrustAnchor::new(
            vec![
                source_for_contract(&a, &first, height, first_acc.clone()),
                source_for_contract(&b, &first, height, first_acc.clone()),
            ],
            trusted.clone(),
            2,
            mock_chain_id(),
            first,
        )
        .unwrap();

        let second_anchor = AttestedTrustAnchor::new(
            vec![
                source_for_contract(&a, &second, height, second_acc.clone()),
                source_for_contract(&b, &second, height, second_acc.clone()),
            ],
            trusted,
            2,
            mock_chain_id(),
            second,
        )
        .unwrap();

        // same type, differing only in the contract supplied at construction: each resolves
        // its own digest rather than the other's
        assert_eq!(
            first_anchor
                .trusted_digest(height)
                .await
                .unwrap()
                .accumulator,
            first_acc
        );
        assert_eq!(
            second_anchor
                .trusted_digest(height)
                .await
                .unwrap()
                .accumulator,
            second_acc
        );
    }

    #[tokio::test]
    async fn a_snapshot_naming_another_contract_is_rejected_before_quorum_counting() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let height = Height::from(100u32);

        let anchored = mock_contract(0);
        let other = mock_contract(1);

        // both signers are trusted and both signatures are valid - the ONLY thing wrong is
        // that the snapshots are scoped to a different contract
        let anchor = AttestedTrustAnchor::new(
            vec![
                source_for_contract(&a, &other, height, LtHash16::new()),
                source_for_contract(&b, &other, height, LtHash16::new()),
            ],
            trusted,
            2,
            mock_chain_id(),
            anchored,
        )
        .unwrap();

        // `agreed: 0` is the point: the wrong-contract snapshots were filtered out before
        // being grouped, so they never counted towards the quorum in the first place. This
        // is what makes a per-domain signing-payload tag unnecessary - the contract address
        // bound into the payload already separates them.
        let err = anchor.refresh().await.unwrap_err();
        assert!(matches!(
            err,
            AnchorError::QuorumNotReached {
                needed: 2,
                agreed: 0
            }
        ));
    }
}
