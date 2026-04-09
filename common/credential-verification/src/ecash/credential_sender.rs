// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Error;
use crate::ecash::error::EcashTicketError;
use crate::ecash::state::SharedState;
use cosmwasm_std::Fraction;
use cw_utils::ThresholdResponse;
use futures::channel::mpsc::UnboundedReceiver;
use futures::{Stream, StreamExt};
use nym_api_requests::ecash::models::VerifyEcashTicketBody;
use nym_credentials_interface::Bandwidth;
use nym_credentials_interface::{ClientTicket, TicketType};
use nym_validator_client::EcashApiClient;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::MultisigQueryClient;
use si_scale::helpers::bibytes2;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLockReadGuard};
use tokio::time::{Duration, Instant, interval_at};
use tracing::{debug, error, info, instrument, trace, warn};

struct PendingVerification {
    ticket: ClientTicket,

    // vec of node ids of apis that haven't sent a valid response
    pending: Vec<u64>,
}

impl PendingVerification {
    fn new(ticket: ClientTicket, pending: Vec<u64>) -> Self {
        PendingVerification { ticket, pending }
    }

    fn to_request_body(&self, gateway_cosmos_addr: AccountId) -> VerifyEcashTicketBody {
        VerifyEcashTicketBody {
            // TODO: redundant clone
            credential: self.ticket.spending_data.clone(),
            gateway_cosmos_addr,
        }
    }
}

pub struct CredentialHandlerConfig {
    /// Specifies the multiplier for revoking a malformed/double-spent ticket
    /// (if it has to go all the way to the nym-api for verification)
    /// e.g. if one ticket grants 100Mb and `revocation_bandwidth_penalty` is set to 1.5,
    /// the client will lose 150Mb
    pub revocation_bandwidth_penalty: f32,

    /// Specifies the interval for attempting to resolve any failed, pending operations,
    /// such as ticket verification or redemption.
    pub pending_poller: Duration,

    pub minimum_api_quorum: f32,

    /// Specifies the minimum number of tickets this gateway will attempt to redeem.
    pub minimum_redemption_tickets: usize,

    /// Specifies the maximum time between two subsequent tickets redemptions.
    /// That's required as nym-apis will purge all ticket information for tickets older than maximum validity.
    pub maximum_time_between_redemption: Duration,
}

pub struct CredentialHandler {
    config: CredentialHandlerConfig,
    multisig_threshold: f32,
    ticket_receiver: UnboundedReceiver<ClientTicket>,
    shared_state: SharedState,
    pending_tickets: Vec<PendingVerification>,
}

impl CredentialHandler {
    async fn rebuild_pending_tickets(
        shared_state: &SharedState,
    ) -> Result<Vec<PendingVerification>, EcashTicketError> {
        // 1. get all tickets that were not fully verified
        let unverified = shared_state.storage.get_all_unverified_tickets().await?;
        let mut pending = Vec::with_capacity(unverified.len());

        // a lookup of ids for signers for given epoch
        let mut apis_lookup = HashMap::new();

        // 2. for each of them, reconstruct missing votes
        for ticket in unverified {
            let epoch = ticket.spending_data.epoch_id;
            assert!(epoch <= i64::MAX as u64);
            let signers = match apis_lookup.get(&epoch) {
                Some(signers) => signers,
                None => {
                    // get all signers for given epoch
                    let signers = shared_state.storage.get_signers(epoch as i64).await?;
                    apis_lookup.insert(epoch, signers);

                    // safety: we just inserted that entry
                    #[allow(clippy::unwrap_used)]
                    apis_lookup.get(&epoch).unwrap()
                }
            };
            // get all votes the ticket received
            let votes = shared_state
                .storage
                .get_votes(ticket.ticket_id)
                .await?
                .into_iter()
                .collect::<HashSet<_>>();
            let mut missing_votes = Vec::new();
            for signer in signers {
                // for each signer, check if they have actually voted; if not, that's the missing guy
                if !votes.contains(signer) {
                    missing_votes.push(*signer as u64)
                }
            }
            pending.push(PendingVerification {
                ticket,
                pending: missing_votes,
            })
        }
        Ok(pending)
    }

    pub(crate) async fn new(
        config: CredentialHandlerConfig,
        ticket_receiver: UnboundedReceiver<ClientTicket>,
        shared_state: SharedState,
    ) -> Result<Self, Error> {
        let multisig_threshold = shared_state
            .nyxd_client
            .read()
            .await
            .query_threshold()
            .await?;

        let ThresholdResponse::AbsolutePercentage { percentage, .. } = multisig_threshold else {
            return Err(Error::InvalidMultisigThreshold);
        };

        // that's a nasty conversion, but it works : )
        let multisig_threshold =
            percentage.numerator().u128() as f32 / percentage.denominator().u128() as f32;

        // on startup read pending credentials and api responses from the storage
        let pending_tickets = Self::rebuild_pending_tickets(&shared_state).await?;

        Ok(CredentialHandler {
            config,
            multisig_threshold,
            ticket_receiver,
            shared_state,
            pending_tickets,
        })
    }

    /// Attempt to send ticket verification request to the provided ecash verifier.
    async fn verify_ticket(
        &self,
        ticket_id: i64,
        request: &VerifyEcashTicketBody,
        client: &EcashApiClient,
    ) -> Result<bool, EcashTicketError> {
        match client.api_client.verify_ecash_ticket(request).await {
            Ok(res) => {
                let accepted = match res.verified {
                    Ok(_) => {
                        trace!("{client} has accepted ticket {ticket_id}");
                        true
                    }
                    Err(rejection) => {
                        warn!("{client} has rejected ticket {ticket_id}: {rejection}");
                        false
                    }
                };
                self.shared_state
                    .storage
                    .insert_ticket_verification(
                        ticket_id,
                        client.node_id as i64,
                        OffsetDateTime::now_utc(),
                        accepted,
                    )
                    .await?;
                Ok(accepted)
            }
            Err(err) => {
                error!(
                    "failed to send ticket {ticket_id} for verification to ecash signer '{client}': {err}. if we don't reach quorum, we'll retry later"
                );
                Err(EcashTicketError::ApiFailure(EcashApiError::NymApi {
                    source: nym_validator_client::ValidatorClientError::from(err),
                }))
            }
        }
    }

    #[instrument(skip(self))]
    async fn revoke_ticket_bandwidth(
        &self,
        ticket_id: i64,
        ticket_type: TicketType,
    ) -> Result<(), EcashTicketError> {
        warn!("revoking bandwidth associated with ticket {ticket_id} since it failed verification");

        let bytes_to_revoke = Bandwidth::ticket_amount(ticket_type.into()).value() as f32
            * self.config.revocation_bandwidth_penalty;
        let to_revoke_bi2 = bibytes2(bytes_to_revoke as f64);

        info!(to_revoke_bi2);

        self.shared_state
            .storage
            .revoke_ticket_bandwidth(ticket_id, bytes_to_revoke as i64)
            .await?;
        Ok(())
    }

    /// Attempt to send the pending ticket to all ecash verifiers that haven't yet returned valid response.
    async fn send_pending_ticket_for_verification(
        &self,
        pending: &mut PendingVerification,
        api_clients: Option<RwLockReadGuard<'_, Vec<EcashApiClient>>>,
    ) -> Result<bool, EcashTicketError> {
        let ticket_id = pending.ticket.ticket_id;
        let ticket_type =
            TicketType::try_from_encoded(pending.ticket.spending_data.payment.t_type)?;
        let api_clients = match api_clients {
            Some(clients) => clients,
            None => {
                self.shared_state
                    .api_clients(pending.ticket.spending_data.epoch_id)
                    .await?
            }
        };

        let verification_request = pending.to_request_body(self.shared_state.address.clone());

        let total = api_clients.len();
        let api_failures = Mutex::new(Vec::new());
        let rejected = AtomicUsize::new(0);

        // this vector will never contain more than ~30 entries so linear lookup is fine.
        // it's probably even faster than hashset due to overhead
        futures::stream::iter(
            api_clients
                .deref()
                .iter()
                .filter(|client| pending.pending.contains(&client.node_id)),
        )
        .for_each_concurrent(32, |ecash_client| async {
            // errors are only returned on hard, storage, failures
            match self
                .verify_ticket(
                    pending.ticket.ticket_id,
                    &verification_request,
                    ecash_client,
                )
                .await
            {
                Err(err) => {
                    error!("internal failure. could not proceed with ticket verification: {err}");
                    api_failures.lock().await.push(ecash_client.node_id);
                }
                Ok(false) => {
                    rejected.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        })
        .await;

        let api_failures = api_failures.into_inner();
        let num_failures = api_failures.len();
        pending.pending = api_failures;

        let rejected = rejected.into_inner();
        let rejected_ratio = rejected as f32 / total as f32;
        let rejected_perc = rejected_ratio * 100.;
        if rejected_ratio >= (1. - self.config.minimum_api_quorum) {
            error!(
                "{rejected_perc:.2}% of signers rejected ticket {ticket_id}. we won't be able to redeem it"
            );

            self.shared_state
                .storage
                .update_rejected_ticket(pending.ticket.ticket_id)
                .await?;
            self.revoke_ticket_bandwidth(pending.ticket.ticket_id, ticket_type)
                .await?;
        }

        let accepted_ratio = (total - rejected - num_failures) as f32 / total as f32;
        let accepted_perc = accepted_ratio * 100.;
        match accepted_ratio {
            n if n < self.multisig_threshold => error!(
                "less than 2/3 of signers ({accepted_perc:.2}%) accepted ticket {ticket_id}. we won't be able to spend it"
            ),
            n if n < self.config.minimum_api_quorum => warn!(
                "less than 80%, but more than 67% of signers ({accepted_perc:.2}%) accepted ticket {ticket_id}. technically we could redeem it, but we'll wait for the bigger quorum"
            ),
            _ => {
                trace!("{accepted_perc:.2}% of signers accepted ticket {ticket_id}");
                self.shared_state
                    .storage
                    .update_verified_ticket(pending.ticket.ticket_id)
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn send_ticket_for_verification(
        &mut self,
        ticket: ClientTicket,
    ) -> Result<(), EcashTicketError> {
        let api_clients = self
            .shared_state
            .api_clients(ticket.spending_data.epoch_id)
            .await?;

        let ids = api_clients.iter().map(|c| c.node_id).collect();
        let mut pending = PendingVerification::new(ticket, ids);

        let got_quorum = self
            .send_pending_ticket_for_verification(&mut pending, Some(api_clients))
            .await?;
        if !got_quorum {
            debug!(
                "failed to reach quorum for ticket {}. apis: {:?} haven't responded. we'll retry later",
                pending.ticket.ticket_id, pending.pending
            );
            self.pending_tickets.push(pending);
        } else {
            // since we reached the quorum we no longer need to hold the ticket's binary data
            self.shared_state
                .storage
                .remove_verified_ticket_binary_data(pending.ticket.ticket_id)
                .await?;
        }

        Ok(())
    }

    async fn handle_client_ticket(&mut self, ticket: ClientTicket) {
        // attempt to send for verification
        let ticket_id = ticket.ticket_id;
        if let Err(err) = self.send_ticket_for_verification(ticket).await {
            error!("failed to verify ticket {ticket_id}: {err}")
        }
    }

    async fn resolve_pending(&mut self) -> Result<(), EcashTicketError> {
        let mut still_failing = Vec::new();

        // 1. attempt to verify the remaining tickets
        while let Some(mut pending) = self.pending_tickets.pop() {
            // possible optimisation: if there's a lot of pending tickets, pre-emptively grab locks for api_clients
            match self
                .send_pending_ticket_for_verification(&mut pending, None)
                .await
            {
                Ok(got_quorum) => {
                    if !got_quorum {
                        warn!(
                            "still failed to reach quorum for ticket {}. apis: {:?} haven't responded. we'll retry later",
                            pending.ticket.ticket_id, pending.pending
                        );
                        still_failing.push(pending);
                    } else {
                        // since we reached the quorum we no longer need to hold the ticket's binary data
                        self.shared_state
                            .storage
                            .remove_verified_ticket_binary_data(pending.ticket.ticket_id)
                            .await?;
                    }
                }
                Err(err) => {
                    error!(
                        "experienced internal error when attempting to resolve pending ticket: {err}"
                    );
                    // make sure to update internal state to not lose any data
                    self.pending_tickets.push(pending);
                    self.pending_tickets.append(&mut still_failing);
                    return Err(err);
                }
            }
        }
        // at this point self.pending_tickets is empty
        self.pending_tickets = still_failing;
        Ok(())
    }

    async fn periodic_operations(&mut self) -> Result<(), EcashTicketError> {
        trace!(
            "attempting to resolve all pending operations -> tickets that are waiting for verification"
        );

        // retry the pending verification requests that have failed before
        self.resolve_pending().await?;

        Ok(())
    }

    pub async fn run(mut self, shutdown: nym_task::ShutdownToken) {
        info!("Starting Ecash CredentialSender");

        // attempt to clear any pending operations
        info!("attempting to resolve any pending operations");
        if let Err(err) = self.periodic_operations().await {
            error!("failed to resolve pending operations on startup: {err}")
        }

        let start = Instant::now() + self.config.pending_poller;
        let mut resolver_interval = interval_at(start, self.config.pending_poller);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => {
                    trace!("client_handling::credentialSender : received shutdown");
                    break
                },
                Some(ticket) = self.ticket_receiver.next() => {
                    let (queued_up, _) = self.ticket_receiver.size_hint();

                    // this will help us determine if we need to parallelize it
                    match queued_up {
                        n if n < 5 => debug!("there are {n} tickets queued up that need processing"),
                        n if (5..20).contains(&n) => info!("there are {n} tickets queued up that need processing"),
                        n if (20..50).contains(&n) => warn!("there are {n} tickets queued up that need processing!"),
                        n => error!("there are {n} tickets queued up that need processing!"),
                    }

                    self.handle_client_ticket(ticket).await
                },
                _ = resolver_interval.tick() => {
                    if let Err(err) = self.periodic_operations().await {
                        error!("failed to deal with periodic operations: {err}")
                    }
                }
            }
        }
    }
}
