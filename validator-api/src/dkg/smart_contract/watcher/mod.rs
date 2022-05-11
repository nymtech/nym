// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::dkg::state::StateAccessor;
use crate::Client;
use coconut_dkg_common::types::EpochState;
use log::warn;
use std::time::Duration;
use tokio::time::interval;

pub(crate) use event::{Event, EventType};
use validator_client::nymd::SigningCosmWasmClient;

mod event;

pub(crate) struct Watcher<C> {
    client: Client<C>,
    polling_rate: Duration,
    state_accessor: StateAccessor,
}

impl<C> Watcher<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    async fn try_get_new_dealers(&self) -> Result<(), DkgError> {
        let known_dealers = self.state_accessor.get_known_dealers().await;
        let bad_dealers = self.state_accessor.get_malformed_dealers().await;

        let current_height = self.client.current_block_height().await?.value();
        let current_dealers = self.client.get_current_dealers().await?;
        for dealer in current_dealers {
            let is_bad = bad_dealers.contains(&dealer.address);
            // ideally we would have been accessing the hashmap by address, but it would make
            // other parts inconvenient. However, we're extremely unlikely to have more than 100 dealers
            // anyway, so the overhead of iterating over the entire map is minimal
            let is_known = known_dealers
                .values()
                .any(|known_dealer| known_dealer.chain_address == dealer.address);

            // we had absolutely no idea about this dealer existing
            if !is_bad || !is_known {
                let watcher_event = self::Event::new(
                    current_height,
                    EventType::NewDealerIdentity { details: dealer },
                );
                self.state_accessor
                    .push_contract_change_event(watcher_event)
                    .await;
            }
        }

        Ok(())
    }

    async fn perform_epoch_state_based_actions(&self, state: EpochState) -> Result<(), DkgError> {
        match state {
            EpochState::PublicKeySubmission { .. } => self.try_get_new_dealers().await,
            EpochState::DealingExchange { .. } => todo!(),
            EpochState::ComplaintSubmission { .. } => todo!(),
            EpochState::ComplaintVoting { .. } => todo!(),
            EpochState::VerificationKeySubmission { .. } => todo!(),
            EpochState::VerificationKeyMismatchSubmission { .. } => todo!(),
            EpochState::VerificationKeyMismatchVoting { .. } => todo!(),
            EpochState::InProgress { .. } => todo!(),
        }
    }

    async fn poll_contract(&self) -> Result<(), DkgError> {
        // based on the current epoch state (assuming it HASN'T CHANGED since last check), the following further actions have to be performed:
        // (if the epoch state changed, we have to ALSO perform actions as if it was in the previous variants):

        // 1. PublicKeySubmission -> get keys of all submitted dealers and if there are any new ones, update dkg state
        // 2. DealingExchange -> get commitments to dealings and if there are any new ones, update dkg state
        // 3. ComplaintSubmission -> look for any complaints and if there is any, grab it and emit an event
        // 4. ComplaintVoting -> grab information about any votes and if exist, emit an event
        // 5. VerificationKeySubmission -> get list of who submitted their verification keys and if there are any new ones either update state or emit an event
        // 6. VerificationKeyMismatchSubmission -> look for any complaints and if there is any, grab it and emit an event,
        // 7. VerificationKeyMismatchVoting -> grab information about any votes and if exist, emit an event
        // 8. InProgress -> No need to do anything (... I think? unless maybe there was any information about epoch transition, to be determined)

        // figure out what we need to pay attention to (for example if we're in "waiting for complaints" state,
        // we don't care about identities of potential new dealers just yet)
        let prior_epoch = self.state_accessor.current_epoch().await;
        let current_epoch = self.client.get_dkg_epoch().await?;

        if prior_epoch.state != current_epoch.state {
            todo!()
        }

        Ok(())
    }

    pub(crate) async fn run(&self) {
        let mut interval = interval(self.polling_rate);
        loop {
            interval.tick().await;
            if let Err(err) = self.poll_contract().await {
                warn!(
                    "failed to get the current state of the DKG contract - {}",
                    err
                )
            }
        }
    }
}
