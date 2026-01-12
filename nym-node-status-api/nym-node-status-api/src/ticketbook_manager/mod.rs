// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ticketbook_manager::helpers::build_sha_short;
use crate::ticketbook_manager::state::TicketbookManagerState;
use futures_util::StreamExt;
use nym_credential_proxy_lib::deposits_buffer::{
    BufferedDeposit, PerformedDeposits, make_deposits_request, split_deposits,
};
use nym_credential_proxy_lib::shared_state::ecash_state::{
    IssuanceTicketBook, IssuedTicketBook, TicketType,
};
use nym_credentials::obtain_aggregate_wallet;
use nym_ecash_time::ecash_default_expiration_date;
use nym_task::ShutdownToken;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::Date;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tracing::{error, info, warn};

mod helpers;
pub(crate) mod state;
pub(crate) mod storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketbookManagerConfig {
    /// Determines how often we should check whether additional ticketbooks should get requested
    pub(crate) check_interval: Duration,

    /// Number of tickets to keep in reserve (per type)
    pub(crate) tickets_buffer_size: usize,

    /// Maximum number of ticketbook deposits that can be made in a single transaction
    pub(crate) max_concurrent_deposits: usize,

    /// Types of ticketbooks to keep in reserve
    // (for example we don't need `V1MixnetExit` tickets
    pub(crate) buffered_ticket_types: Vec<TicketType>,
}

impl Default for TicketbookManagerConfig {
    fn default() -> Self {
        use TicketType::*;
        TicketbookManagerConfig {
            check_interval: Duration::from_secs(60),
            tickets_buffer_size: 50,
            max_concurrent_deposits: 32,
            buffered_ticket_types: vec![V1MixnetEntry, V1WireguardEntry, V1WireguardExit],
        }
    }
}

pub struct TicketbookManager {
    config: TicketbookManagerConfig,
    short_sha: &'static str,
    ecash_key_identifier: Vec<u8>,

    shutdown_token: ShutdownToken,
    state: TicketbookManagerState,
}

impl TicketbookManager {
    pub(crate) fn new(
        config: TicketbookManagerConfig,
        state: TicketbookManagerState,
        ecash_key_identifier: Vec<u8>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        TicketbookManager {
            config,
            short_sha: build_sha_short(),
            ecash_key_identifier,
            shutdown_token,
            state,
        }
    }

    async fn get_ticketbooks_from_deposits(
        &self,
        deposits: PerformedDeposits,
        ticket_type: TicketType,
    ) -> anyhow::Result<()> {
        let expiration_date = ecash_default_expiration_date();
        let epoch_id = self.state.current_epoch_id().await?;
        let threshold = self.state.ecash_threshold(epoch_id).await?;
        let ecash_clients = self.state.ecash_clients(epoch_id).await?;

        // ensure we have required global data in case the epoch has rolled over or a new day has begun
        let _ = self
            .state
            .master_expiration_date_signatures(epoch_id, expiration_date)
            .await?;
        let _ = self
            .state
            .master_coin_index_signatures(Some(epoch_id))
            .await?;
        let _ = self.state.master_verification_key(Some(epoch_id)).await?;

        let total = deposits.deposits_data.len();
        for (i, deposit) in deposits.deposits_data.into_iter().enumerate() {
            info!(
                "getting ticketbook {} / {total} from this deposit batch",
                i + 1
            );
            let issuance_data =
                self.deposit_to_issuance_ticketbook(deposit, ticket_type, expiration_date);
            let aggregated_wallet = match obtain_aggregate_wallet(
                &issuance_data,
                &ecash_clients,
                threshold,
            )
            .await
            {
                Err(err) => {
                    error!("failed to obtain aggregated wallet: {err}");
                    self.state
                        .storage()
                        .insert_pending_ticketbook(&issuance_data).await.inspect_err(|err| {
                            let deposit = issuance_data.deposit_id();
                            error!("could not save the recovery data for deposit {deposit}: {err}. the data will unfortunately get lost")
                        })?;
                    return Err(err.into());
                }
                Ok(wallet) => wallet,
            };
            let ticketbook = issuance_data.to_issued_ticketbook(aggregated_wallet, epoch_id);
            self.state
                .storage()
                .insert_issued_ticketbook(&ticketbook)
                .await?;
        }

        Ok(())
    }

    async fn make_deposits(&self, amount: usize) -> anyhow::Result<PerformedDeposits> {
        info!("performing {amount} deposits");
        let deposit_amount = self.state.deposit_amount().await?;

        let memo = format!(
            "node-status-api-proxy-{}: performing {amount} deposits",
            self.short_sha
        );
        let deposits = make_deposits_request(
            self.state.client(),
            deposit_amount.clone(),
            &memo,
            amount,
            self.shutdown_token.inner(),
        )
        .await?;

        Ok(deposits)
    }

    fn deposit_to_issuance_ticketbook(
        &self,
        deposit: BufferedDeposit,
        ticket_type: TicketType,
        expiration_date: Date,
    ) -> IssuanceTicketBook {
        IssuanceTicketBook::new_with_expiration(
            deposit.deposit_id,
            &self.ecash_key_identifier,
            deposit.ed25519_private_key,
            ticket_type,
            expiration_date,
        )
    }

    async fn maybe_refill_ticketbook(&self, ticket_type: TicketType) -> anyhow::Result<()> {
        let available_tickets = self
            .state
            .storage()
            .available_tickets_of_type(ticket_type)
            .await?;

        info!("{ticket_type}: {available_tickets} available tickets");
        if available_tickets >= self.config.tickets_buffer_size {
            info!("no need to request additional ticketbooks");
            // nothing to do
            return Ok(());
        }
        info!(
            "we're below {} threshold - going to get a new ticketbook",
            self.config.tickets_buffer_size
        );

        let needed_tickets = self.config.tickets_buffer_size - available_tickets;
        let tickets_per_ticketbook = IssuedTicketBook::global_total_tickets() as usize;

        // use ceil division, so that if for example we need 9 tickets, we'd still get the whole ticketbook (10 tickets)
        let needed_ticketbooks = needed_tickets.div_ceil(tickets_per_ticketbook);

        for request_chunk in split_deposits(needed_ticketbooks, self.config.max_concurrent_deposits)
        {
            // 1. check for cancellation
            // note: we check for cancellation between individual requests
            // as opposed to wrapping that in tokio::select! so that we would never abandon chain operations
            // as we wouldn't want to lose funds
            if self.shutdown_token.is_cancelled() {
                info!("received cancellation during ticketbook refilling");
                return Ok(());
            }
            // 2. make required deposits
            let deposits = self.make_deposits(request_chunk).await?;

            // TODO: in the future we should persist deposits before requesting ticketbooks
            // in case of failures

            // 3. for each deposit get the corresponding ticketbook and persist them in the storage
            self.get_ticketbooks_from_deposits(deposits, ticket_type)
                .await?;
        }

        Ok(())
    }

    async fn check_ticketbooks_buffer(&self) {
        if let Err(err) = self.state.ensure_credentials_issuable().await {
            warn!("ticketbooks can't be issued at this time: {err}");
            return;
        }
        if !self.state.ecash_state().quorum_state.available() {
            error!("can't refill our ticketbooks as signing quorum is not available");
            return;
        }

        for ticket_type in &self.config.buffered_ticket_types {
            if let Err(err) = self.maybe_refill_ticketbook(*ticket_type).await {
                error!("failed to refill {ticket_type} ticketbooks: {err}")
            }
        }
    }

    pub async fn run(&self) {
        // given we're not going to be using up all ticketbooks in a span of few seconds,
        // just run periodically and refill our buffer
        let mut check_stream = IntervalStream::new(interval(self.config.check_interval));

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    tracing::trace!("TicketbookManager: Received shutdown");
                    break;
                }
                _ = check_stream.next() => {
                    self.check_ticketbooks_buffer().await;
                }
            }
        }
    }
}
