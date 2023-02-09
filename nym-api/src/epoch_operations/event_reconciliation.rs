// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_operations::error::RewardingError;
use crate::RewardedSetUpdater;

impl RewardedSetUpdater {
    pub(super) async fn reconcile_epoch_events(&self) -> Result<(), RewardingError> {
        let pending_events = self.nyxd_client.get_pending_events_count().await?;
        // if there's no pending events, job's done.
        if pending_events == 0 {
            return Ok(());
        }

        // be conservative about number of events we resolve at once.
        // contract should be able to handle few hundred at once
        // but keep it on the lower side.
        let limit = 100;

        let mut required_calls = pending_events / limit;
        if pending_events % limit != 0 {
            required_calls += 1;
        }

        for _ in 0..required_calls {
            self.nyxd_client.reconcile_epoch_events(Some(limit)).await?;
        }

        // in the incredibly unlikely/borderline impossible scenario a HUGE number of events got pushed
        // while we were reconciling the events, do it one more time
        //
        // note: it's perfectly fine if we don't clear EXACTLY everything,
        // since when we execute transaction to actually advance the epoch,
        // it will resolve all remaining events.
        let pending_events = self.nyxd_client.get_pending_events_count().await?;
        if pending_events > 20 {
            self.nyxd_client.reconcile_epoch_events(Some(limit)).await?;
        }

        Ok(())
    }
}
