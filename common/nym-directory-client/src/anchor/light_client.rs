// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::helpers::get_trusted_directory_digest;
use crate::anchor::{Checkpoint, DirectoryTrustAnchor, TrustedDigest};
use crate::error::DirectoryClientError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::AppHash;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt, ValidatorSet};
use std::collections::BTreeMap;
use std::time::Duration;
use tendermint_light_client::light_client::Options;
use tendermint_light_client::types::{
    Hash, SignedHeader, Time, TrustThreshold, TrustedBlockState, UntrustedBlockState,
};
use tendermint_light_client::verifier::{ProdVerifier, Verdict, Verifier};
use tokio::sync::Mutex;
use tracing::debug;

/// Sane defaults for the Nym mainnet: trust threshold 1/3 (required for skip/bisection
/// verification), trusting period of 14 days (below the 21-day unbonding period), and
/// a 5-second clock-drift allowance.
pub fn nyx_default_options() -> Options {
    Options {
        trust_threshold: TrustThreshold::ONE_THIRD,
        trusting_period: Duration::from_secs(14 * 24 * 60 * 60),
        clock_drift: Duration::from_secs(5),
    }
}

/// Owned state from which `TrustedBlockState<'_>` is constructed on demand.
#[derive(Clone)]
struct TrustedAnchorState {
    chain_id: cosmrs::tendermint::chain::Id,
    header_time: Time,
    height: Height,
    next_validators: ValidatorSet,
    next_validators_hash: Hash,
}

impl From<Checkpoint> for TrustedAnchorState {
    fn from(checkpoint: Checkpoint) -> Self {
        TrustedAnchorState {
            chain_id: checkpoint.signed_header.header.chain_id,
            header_time: checkpoint.signed_header.header.time,
            height: checkpoint.height,
            next_validators_hash: checkpoint.signed_header.header.next_validators_hash,
            next_validators: checkpoint.next_validators,
        }
    }
}

impl TrustedAnchorState {
    fn as_trusted_block_state(&self) -> TrustedBlockState<'_> {
        TrustedBlockState {
            chain_id: &self.chain_id,
            header_time: self.header_time,
            height: self.height,
            next_validators: &self.next_validators,
            next_validators_hash: self.next_validators_hash,
        }
    }

    fn advance(&mut self, signed_header: SignedHeader, next_validators: ValidatorSet) {
        self.chain_id = signed_header.header.chain_id.clone();
        self.header_time = signed_header.header.time;
        self.height = signed_header.header.height;
        self.next_validators_hash = signed_header.header.next_validators_hash;
        self.next_validators = next_validators;
    }
}

struct LightClientAnchorState {
    /// The immutable pinned checkpoint. Used as the base for verifying heights the advancing
    /// head has already passed (the verifier only moves forward, so we cannot re-verify a
    /// below-head height from `trusted`).
    checkpoint: TrustedAnchorState,

    /// The furthest-verified state, advanced forward by monotonically-increasing queries.
    trusted: TrustedAnchorState,

    /// Cache: query height `H` -> `header[H+1].app_hash` (the app state committed at `H`).
    app_hash_cache: BTreeMap<Height, AppHash>,
}

pub struct LightClientAnchor<C> {
    client: C,

    directory_contract: AccountId,

    // we only need Mutex to be able to take &self without mutable reference
    // there's no concurrent access anywhere
    state: Mutex<LightClientAnchorState>,

    options: Options,

    verifier: ProdVerifier,
}

impl<C> LightClientAnchor<C> {
    pub fn new(
        client: C,
        directory_contract: AccountId,
        checkpoint: Checkpoint,
        options: Options,
    ) -> Self {
        let mut app_hash_cache = BTreeMap::new();
        // the checkpoint's own header commits state at `checkpoint.height - 1`, so we can serve
        // that one height directly without any verification.
        if checkpoint.height.value() > 1 {
            app_hash_cache.insert(
                Height::from(checkpoint.height.value() as u32 - 1),
                checkpoint.signed_header.header.app_hash.clone(),
            );
        }
        let trusted: TrustedAnchorState = checkpoint.into();
        Self {
            client,
            directory_contract,
            state: Mutex::new(LightClientAnchorState {
                checkpoint: trusted.clone(),
                trusted,
                app_hash_cache,
            }),
            options,
            verifier: ProdVerifier::default(),
        }
    }
}

impl<C> LightClientAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    /// Verify the header at `target` directly against `base` via the Tendermint light-client rule.
    ///
    /// Returns `Some((signed_header, next_validators))` on success (`next_validators` is the
    /// set at `target + 1`, ready to become the new trusted state's next-validators),
    /// `None` when the trusted validator overlap is insufficient (caller should bisect),
    /// `Err` on hard verification failures or RPC errors.
    async fn verify_hop(
        &self,
        base: &TrustedAnchorState,
        target: Height,
    ) -> Result<Option<(SignedHeader, ValidatorSet)>, DirectoryClientError> {
        let commit_res = self.client.commit(target).await?;
        if !commit_res.canonical {
            return Err(DirectoryClientError::NonCanonicalCommit(target.value()));
        }
        let validators = ValidatorSet::without_proposer(
            self.client.get_all_validators(target).await?.validators,
        );
        // the new trusted state at `target` must carry the validator set of `target + 1` as its
        // next-validators (that is what skip verification checks the next commit's overlap against).
        let next = Height::from(target.value() as u32 + 1);
        let next_validators =
            ValidatorSet::without_proposer(self.client.get_all_validators(next).await?.validators);

        // pass `next_validators` so the verifier ties it to the verified header's
        // `next_validators_hash` (`next_validators_match`); otherwise the RPC-supplied set we
        // store for the next skip hop would be trusted blindly.
        let untrusted = UntrustedBlockState {
            signed_header: &commit_res.signed_header,
            validators: &validators,
            next_validators: Some(&next_validators),
        };
        let trusted = base.as_trusted_block_state();
        let now = Time::now();

        match self
            .verifier
            .verify_update_header(untrusted, trusted, &self.options, now)
        {
            Verdict::Success => Ok(Some((commit_res.signed_header, next_validators))),
            Verdict::NotEnoughTrust(_) => Ok(None),
            Verdict::Invalid(err) => Err(DirectoryClientError::LightClientVerificationFailed(
                err.to_string(),
            )),
        }
    }

    /// Advance `base` forward to `target` using skip verification with bisection, caching every
    /// verified header's app hash into `cache` along the way.
    ///
    /// Attempts to verify `target` directly (O(1) for a stable validator set). On
    /// `NotEnoughTrust` it bisects: verifies the midpoint, advances `base` to it in place, then
    /// retries the target. Depth is O(log(target - base)). `base` is `&mut` so the below-head
    /// walk (over a local checkpoint clone) makes progress without touching the persisted head.
    async fn walk_to(
        &self,
        base: &mut TrustedAnchorState,
        cache: &mut BTreeMap<Height, AppHash>,
        target: Height,
    ) -> Result<(), DirectoryClientError> {
        let current = base.height;
        if current >= target {
            return Ok(());
        }
        debug!("light-client: advancing from {current} to {target}",);

        if let Some((signed_header, next_validators)) = self.verify_hop(base, target).await? {
            // `header[target]` commits the app state at `target - 1` (CometBFT off-by-one); this
            // holds for any verified header, including bisection midpoints.
            cache.insert(
                Height::from(target.value() as u32 - 1),
                signed_header.header.app_hash.clone(),
            );
            base.advance(signed_header, next_validators);
            return Ok(());
        }

        // NotEnoughTrust: bisect.
        let mid = Height::from((current.value() as u32 + target.value() as u32) / 2);
        debug!("light-client: bisecting [{current}, {target}] via midpoint {mid}");
        Box::pin(self.walk_to(base, cache, mid)).await?;
        Box::pin(self.walk_to(base, cache, target)).await
    }

    /// Ensure `header[target]` is verified and its app hash cached.
    ///
    /// Forward of the head: advance the persisted head. At or below the head (a height the head
    /// already passed but never cached): walk a local clone of the checkpoint up to `target`,
    /// since the verifier cannot re-verify backwards from the head.
    async fn advance_to(
        &self,
        state: &mut LightClientAnchorState,
        target: Height,
    ) -> Result<(), DirectoryClientError> {
        if state.trusted.height >= target {
            if target <= state.checkpoint.height {
                return Err(DirectoryClientError::HeightBelowCheckpoint {
                    requested: target.value().saturating_sub(1),
                    checkpoint: state.checkpoint.height.value(),
                });
            }
            let mut local = state.checkpoint.clone();
            self.walk_to(&mut local, &mut state.app_hash_cache, target)
                .await
        } else {
            self.walk_to(&mut state.trusted, &mut state.app_hash_cache, target)
                .await
        }
    }
}

#[async_trait]
impl<C> DirectoryTrustAnchor for LightClientAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, DirectoryClientError> {
        // the app_hash committing state at H lives in header[H+1] (CometBFT off-by-one)
        let target = Height::from(height.value() as u32 + 1);
        let mut state = self.state.lock().await;

        if let Some(cached) = state.app_hash_cache.get(&height) {
            return Ok(cached.clone());
        }

        self.advance_to(&mut state, target).await?;

        state.app_hash_cache.get(&height).cloned().ok_or_else(|| {
            DirectoryClientError::LightClientVerificationFailed(format!(
                "app_hash for height {height} not in cache after advance"
            ))
        })
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError> {
        let app_hash = self.trusted_app_hash(height).await?;

        get_trusted_directory_digest(&self.client, &self.directory_contract, height, app_hash).await
    }
}
