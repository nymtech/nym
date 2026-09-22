// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::{EpochId, Interval};
use time::OffsetDateTime;
use tracing::debug;

/// Whether this reading still describes the epoch in progress.
///
/// An epoch is advanced by a transaction rather than by the clock, so a reading is authoritative
/// exactly until the epoch it names is due to end: up to that moment nothing can have advanced past
/// it, and after it something may have, with only the contract able to say. Taking the deadline from
/// the reading itself is also what keeps the refresh in step with whatever epoch length the chain
/// runs, leaving no cadence to configure.
fn describes_current_epoch(anchor: &Interval, now: OffsetDateTime) -> bool {
    now < anchor.current_epoch_end()
}

/// Reads the contract's interval.
async fn read_interval(client: &(impl MixnetQueryClient + Sync)) -> anyhow::Result<Interval> {
    let interval = client
        .get_current_interval_details()
        .await
        .context("failed to query the mixnet contract for its current interval")?
        .interval;

    debug!(
        mixnet_epoch = interval.current_epoch_absolute_id(),
        epoch_start = %interval.current_epoch_start(),
        epoch_length_secs = interval.epoch_length_secs(),
        "read the mixnet epoch anchor from the contract"
    );
    Ok(interval)
}

/// The orchestrator's view of mixnet epochs, read from the mixnet contract.
///
/// Answers from the reading it holds for as long as that reading still describes the epoch in
/// progress, so materialising a whole epoch costs one query rather than one per node. Once the
/// anchored epoch is over a new reading is taken, and a reading that cannot be taken is an error
/// rather than a fall back onto the old one: a locally guessed boundary that disagrees with the
/// chain files measurements under the wrong epoch, which is unrecoverable once published.
///
/// The identifier is a `mixnet_epoch` wherever it appears, never a bare `epoch`, which the
/// orchestrator's storage already uses for the unrelated sphinx `key_rotation_id`.
pub(crate) struct MixnetEpochSource<C> {
    client: C,

    /// The most recent reading of the contract's interval.
    anchor: Interval,
}

impl<C: MixnetQueryClient + Sync> MixnetEpochSource<C> {
    /// Takes the first reading, so that a source which exists is one that can answer.
    pub(crate) async fn new(client: C) -> anyhow::Result<Self> {
        let anchor = read_interval(&client).await?;
        Ok(MixnetEpochSource { client, anchor })
    }

    /// The absolute id of the mixnet epoch currently in progress, which is the id an aggregate is
    /// filed under.
    pub(crate) async fn current_mixnet_epoch(&mut self) -> anyhow::Result<EpochId> {
        let anchor = self.anchor(OffsetDateTime::now_utc()).await?;
        Ok(anchor.current_epoch_absolute_id())
    }

    /// When `mixnet_epoch` began, which is the point its aggregation window is anchored at.
    pub(crate) async fn mixnet_epoch_start(
        &mut self,
        mixnet_epoch: EpochId,
    ) -> anyhow::Result<OffsetDateTime> {
        let anchor = self.anchor(OffsetDateTime::now_utc()).await?;
        Ok(anchor.epoch_start(mixnet_epoch))
    }

    /// The reading to answer from: the one already held while it still describes the epoch in
    /// progress, a fresh one otherwise.
    ///
    /// A failed refresh leaves the held reading in place but does NOT answer from it: the caller gets
    /// the error, and the epoch it was asking about becomes a backfill candidate for once the chain
    /// is reachable again.
    async fn anchor(&mut self, now: OffsetDateTime) -> anyhow::Result<Interval> {
        if describes_current_epoch(&self.anchor, now) {
            return Ok(self.anchor);
        }

        self.anchor = read_interval(&self.client).await?;
        Ok(self.anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use cosmwasm_std::Timestamp;
    use cosmwasm_std::testing::mock_env;
    use nym_validator_client::nyxd::error::NyxdError;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::QueryMsg as MixnetQueryMsg;
    use serde::Deserialize;
    use std::time::Duration;

    const EPOCH_LENGTH: Duration = Duration::from_secs(60 * 60);
    const EPOCHS_IN_INTERVAL: u32 = 24;

    /// An interval whose epoch began `epochs_ago` epochs before now, i.e. one still running at
    /// `epochs_ago` of zero and over for anything larger.
    fn interval_started(epochs_ago: u32) -> Interval {
        let mut env = mock_env();
        let started_at = OffsetDateTime::now_utc() - epochs_ago * EPOCH_LENGTH;
        env.block.time = Timestamp::from_seconds(started_at.unix_timestamp() as u64);

        Interval::init_interval(EPOCHS_IN_INTERVAL, EPOCH_LENGTH, &env)
    }

    /// A chain that cannot be reached, so that what the source does with the reading it holds is all
    /// that decides the answer.
    struct UnreachableContract;

    #[async_trait]
    impl MixnetQueryClient for UnreachableContract {
        async fn query_mixnet_contract<T>(&self, _query: MixnetQueryMsg) -> Result<T, NyxdError>
        where
            for<'a> T: Deserialize<'a>,
        {
            Err(NyxdError::AbciError {
                code: 1,
                log: "the chain is unreachable".to_string(),
                pretty_log: None,
            })
        }
    }

    fn anchored_at(anchor: Interval) -> MixnetEpochSource<UnreachableContract> {
        MixnetEpochSource {
            client: UnreachableContract,
            anchor,
        }
    }

    // the boundary is the moment the held reading stops speaking for the chain, so it has to fall on
    // the same side as the contract's own `is_current_epoch_over`, which treats an epoch ending
    // exactly now as over
    #[test]
    fn an_anchor_speaks_for_its_own_epoch_only_until_that_epoch_ends() {
        let anchor = interval_started(0);
        let ends_at = anchor.current_epoch_end();

        assert!(describes_current_epoch(
            &anchor,
            ends_at - Duration::from_secs(1)
        ));
        assert!(!describes_current_epoch(&anchor, ends_at));
    }

    // the point of holding a reading at all: materialising an epoch asks repeatedly and queries once
    #[tokio::test]
    async fn an_anchor_still_running_is_answered_from_rather_than_re_read() {
        let mut source = anchored_at(interval_started(0));

        let mixnet_epoch = source
            .current_mixnet_epoch()
            .await
            .expect("a running anchor should have answered without a query");
        assert_eq!(mixnet_epoch, 0);
        assert_eq!(
            source
                .mixnet_epoch_start(0)
                .await
                .expect("a running anchor should have answered without a query"),
            source.anchor.current_epoch_start()
        );
    }

    // extrapolating would have produced a plausible id and a plausible boundary here, and a
    // measurement filed under the wrong epoch cannot be taken back once published
    #[tokio::test]
    async fn a_stale_anchor_is_an_error_rather_than_an_extrapolation() {
        let mut source = anchored_at(interval_started(3));

        assert!(source.current_mixnet_epoch().await.is_err());
        assert!(source.mixnet_epoch_start(0).await.is_err());
    }

    #[tokio::test]
    async fn an_unreachable_contract_yields_no_source_at_all() {
        assert!(MixnetEpochSource::new(UnreachableContract).await.is_err());
    }
}
