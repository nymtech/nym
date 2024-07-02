// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::ecash::error::EcashTicketError;
use crate::node::client_handling::websocket::connection_handler::ecash::helpers::for_each_api_concurrent;
use crate::node::client_handling::websocket::connection_handler::ecash::state::SharedState;
use crate::node::storage::Storage;
use crate::GatewayError;
use futures::channel::mpsc::UnboundedReceiver;
use futures::{Stream, StreamExt};
use log::{debug, error, info, trace, warn};
use nym_api_requests::coconut::models::{BatchRedeemTicketsBody, VerifyEcashTicketBody};
use nym_api_requests::constants::MIN_BATCH_REDEMPTION_DELAY;
use nym_credentials_interface::CredentialSpendingData;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::{
    EcashSigningClient, MultisigQueryClient, MultisigSigningClient, PagedMultisigQueryClient,
};
use nym_validator_client::nyxd::cosmwasm_client::ToSingletonContractData;
use nym_validator_client::nyxd::cw3::Status;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::EcashApiClient;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLockReadGuard};
use tokio::time::{interval_at, Duration, Instant};

// TODO: make those configurable
const RESOLVER_INTERVAL: Duration = Duration::from_secs(300);
const MINIMUM_API_QUORUM: f32 = 0.8;
const MULTISIG_THRESHOLD: f32 = 0.67;

// don't attempt to redeem fewer than that many tickets
const MINIMUM_REDEMPTION_TICKETS: usize = 100;

enum ProposalResult {
    Executed,
    Rejected,
    Pending,
}

impl ProposalResult {
    fn is_pending(&self) -> bool {
        matches!(self, ProposalResult::Pending)
    }

    fn is_rejected(&self) -> bool {
        matches!(self, ProposalResult::Rejected)
    }
}

#[derive(Clone)]
pub struct ClientTicket {
    pub spending_data: CredentialSpendingData,
    pub ticket_id: i64,
}

impl ClientTicket {
    pub fn new(spending_data: CredentialSpendingData, ticket_id: i64) -> Self {
        ClientTicket {
            spending_data,
            ticket_id,
        }
    }
}

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

struct PendingRedemptionVote {
    proposal_id: u64,
    digest: Vec<u8>,
    included_serial_numbers: Vec<Vec<u8>>,
    epoch_id: EpochId,

    // vec of node ids of apis that haven't sent a valid response
    pending: Vec<u64>,
}

impl PendingRedemptionVote {
    fn new(
        proposal_id: u64,
        digest: Vec<u8>,
        included_serial_numbers: Vec<Vec<u8>>,
        epoch_id: EpochId,
        pending: Vec<u64>,
    ) -> Self {
        PendingRedemptionVote {
            proposal_id,
            digest,
            included_serial_numbers,
            epoch_id,
            pending,
        }
    }

    fn to_request_body(&self, gateway_cosmos_addr: AccountId) -> BatchRedeemTicketsBody {
        BatchRedeemTicketsBody::new(
            self.digest.clone(),
            self.proposal_id,
            self.included_serial_numbers.clone(),
            gateway_cosmos_addr,
        )
    }
}

pub(crate) struct CredentialHandler<St: Storage> {
    ticket_receiver: UnboundedReceiver<ClientTicket>,
    shared_state: SharedState<St>,
    pending_tickets: Vec<PendingVerification>,
    pending_redemptions: Vec<PendingRedemptionVote>,
}

impl<St> CredentialHandler<St>
where
    St: Storage + Clone + 'static,
{
    async fn rebuild_pending_tickets(
        shared_state: &SharedState<St>,
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

    async fn rebuild_pending_votes(
        shared_state: &SharedState<St>,
    ) -> Result<Vec<PendingRedemptionVote>, EcashTicketError> {
        // 1. get all tickets that were not fully verified
        let unverified = shared_state.storage.get_all_unresolved_proposals().await?;
        let mut pending = Vec::with_capacity(unverified.len());

        let epoch_id = shared_state.current_epoch_id().await?;
        let apis = shared_state
            .api_clients(epoch_id)
            .await?
            .iter()
            .map(|s| (s.cosmos_address.to_string(), s.node_id))
            .collect::<Vec<_>>();

        for proposal_id in unverified {
            // get all of the votes
            let votes = shared_state
                .start_query()
                .await
                .get_all_votes(proposal_id as u64)
                .await
                .map_err(EcashTicketError::chain_query_failure)?
                .into_iter()
                .map(|v| v.voter)
                .collect::<HashSet<_>>();

            let mut missing_votes = Vec::new();

            // see who hasn't voted
            for (api_address, api_id) in &apis {
                // for each signer, check if they have actually voted; if not, that's the missing guy
                if !votes.contains(api_address) {
                    missing_votes.push(*api_id)
                }
            }

            // attempt to rebuild SN and digest from the proposal info + storage data
            let proposal_info = shared_state
                .start_query()
                .await
                .query_proposal(proposal_id as u64)
                .await
                .map_err(EcashTicketError::chain_query_failure)?;

            let tickets = shared_state
                .storage
                .get_all_proposed_tickets_with_sn(proposal_id as u32)
                .await?;
            let digest =
                BatchRedeemTicketsBody::make_digest(tickets.iter().map(|t| &t.serial_number));
            let encoded_digest = bs58::encode(&digest).into_string();
            if encoded_digest != proposal_info.description {
                error!("the lost proposal {proposal_id} does not have a matching digest!");
                continue;
            }

            pending.push(PendingRedemptionVote {
                proposal_id: proposal_id as u64,
                digest,
                included_serial_numbers: tickets.into_iter().map(|t| t.serial_number).collect(),
                epoch_id,
                pending: missing_votes,
            })
        }

        Ok(pending)
    }

    pub(crate) async fn new(
        ticket_receiver: UnboundedReceiver<ClientTicket>,
        shared_state: SharedState<St>,
    ) -> Result<Self, GatewayError> {
        // on startup read pending credentials and api responses from the storage
        let pending_tickets = Self::rebuild_pending_tickets(&shared_state).await?;

        // on startup read pending proposals from the storage
        // then reconstruct the votes by querying the multisig contract for votes on those proposals
        // digest from the description and count from the message
        let pending_redemptions = Self::rebuild_pending_votes(&shared_state).await?;

        Ok(CredentialHandler {
            ticket_receiver,
            shared_state,
            pending_tickets,
            pending_redemptions,
        })
    }

    // the argument is temporary as we'll be reading from the storage
    async fn create_redemption_proposal(
        &self,
        commitment: &[u8],
        number_of_tickets: u16,
    ) -> Result<u64, EcashTicketError> {
        let res = self
            .shared_state
            .start_tx()
            .await
            .request_ticket_redemption(
                bs58::encode(commitment).into_string(),
                number_of_tickets,
                None,
            )
            .await
            .map_err(|source| EcashTicketError::RedemptionProposalCreationFailure { source })?;

        // that one is quite tricky because proposal exists on chain, but we didn't get the id...
        // but it should be quite impossible to ever reach this unless we make breaking changes
        let proposal_id = res
            .parse_singleton_u64_contract_data()
            .inspect_err(|err| error!("reached seemingly impossible error! could not recover the redemption proposal id: {err}"))
            .map_err(|source| EcashTicketError::ProposalIdParsingFailure { source })?;

        info!("created redemption proposal {proposal_id} to redeem {number_of_tickets} tickets");

        Ok(proposal_id)
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
                error!("failed to send ticket {ticket_id} for verification to ecash signer '{client}': {err}. if we don't reach quorum, we'll retry later");
                Ok(false)
            }
        }
    }

    /// Attempt to send the pending ticket to all ecash verifiers that haven't yet returned valid response.
    async fn send_pending_ticket_for_verification(
        &self,
        pending: &mut PendingVerification,
        api_clients: Option<RwLockReadGuard<'_, Vec<EcashApiClient>>>,
    ) -> Result<bool, EcashTicketError> {
        let ticket_id = pending.ticket.ticket_id;
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
        if rejected_ratio >= (1. - MINIMUM_API_QUORUM) {
            error!("{rejected_ratio:.2} of signers rejected ticket {ticket_id}. we won't be able to redeem it");

            self.shared_state
                .storage
                .update_rejected_ticket(pending.ticket.ticket_id)
                .await?;
        }

        let accepted_ratio = (total - rejected - num_failures) as f32 / total as f32;
        match accepted_ratio {
            n if n < MULTISIG_THRESHOLD => error!("less than 2/3 of signers ({n:.2}%) accepted ticket {ticket_id}. we won't be able to spend it"),
            n if n < MINIMUM_API_QUORUM => warn!("less than 80%, but more than 67% of signers ({n:.2}%) accepted ticket {ticket_id}. technically we could redeem it, but we'll wait for the bigger quorum"),
            n => {
                trace!("{n:.2}% of signers accepted ticket {ticket_id}");
                self.shared_state.storage.update_verified_ticket(pending.ticket.ticket_id).await?;
                return Ok(true)
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
            debug!("failed to reach quorum for ticket {}. apis: {:?} haven't responded. we'll retry later", pending.ticket.ticket_id, pending.pending);
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

        // 1. attempt to resolve all pending proposals
        while let Some(mut pending) = self.pending_redemptions.pop() {
            match self.try_resolve_pending_proposal(&mut pending, None).await {
                Ok(resolution) => {
                    if resolution.is_pending() {
                        warn!("still failed to reach quorum for proposal {}. apis: {:?} haven't responded. we'll retry later", pending.proposal_id, pending.pending);
                        still_failing.push(pending);
                    } else {
                        self.shared_state
                            .storage
                            .clear_post_proposal_data(
                                pending.proposal_id as u32,
                                OffsetDateTime::now_utc(),
                                resolution.is_rejected(),
                            )
                            .await?;
                    }
                }
                Err(err) => {
                    error!("experienced internal error when attempting to resolve pending proposal: {err}");
                    // make sure to update internal state to not lose any data
                    self.pending_redemptions.push(pending);
                    self.pending_redemptions.append(&mut still_failing);
                    return Err(err);
                }
            }
        }

        let mut still_failing = Vec::new();

        // 2. attempt to verify the remaining tickets
        while let Some(mut pending) = self.pending_tickets.pop() {
            // possible optimisation: if there's a lot of pending tickets, pre-emptively grab locks for api_clients
            match self
                .send_pending_ticket_for_verification(&mut pending, None)
                .await
            {
                Ok(got_quorum) => {
                    if !got_quorum {
                        warn!("still failed to reach quorum for ticket {}. apis: {:?} haven't responded. we'll retry later", pending.ticket.ticket_id, pending.pending);
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
                    error!("experienced internal error when attempting to resolve pending ticket: {err}");
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

    /// Attempt to send batch redemption request to the provided ecash verifier.
    async fn redeem_tickets(
        &self,
        proposal_id: u64,
        request: &BatchRedeemTicketsBody,
        client: &EcashApiClient,
    ) -> Result<bool, EcashTicketError> {
        match client.api_client.batch_redeem_ecash_tickets(request).await {
            Ok(res) => {
                let accepted = if res.proposal_accepted {
                    trace!("{client} has accepted proposal {proposal_id}");
                    true
                } else {
                    warn!("{client} has rejected proposal {proposal_id}");
                    false
                };

                Ok(accepted)
            }
            Err(err) => {
                error!("failed to send proposal {proposal_id} for redemption vote to ecash signer '{client}': {err}. if we don't reach quorum, we'll retry later");
                Ok(false)
            }
        }
    }

    async fn try_execute_proposal(&self, proposal_id: u64) -> Result<(), EcashTicketError> {
        self.shared_state
            .start_tx()
            .await
            .execute_proposal(proposal_id, None)
            .await
            .map_err(
                |source| EcashTicketError::RedemptionProposalExecutionFailure {
                    proposal_id,
                    source,
                },
            )?;
        Ok(())
    }

    async fn get_proposal_status(&self, proposal_id: u64) -> Result<Status, EcashTicketError> {
        Ok(self
            .shared_state
            .start_query()
            .await
            .query_proposal(proposal_id)
            .await
            .map_err(EcashTicketError::chain_query_failure)?
            .status)
    }

    async fn try_finalize_proposal(
        &self,
        proposal_id: u64,
    ) -> Result<ProposalResult, EcashTicketError> {
        match self.get_proposal_status(proposal_id).await? {
            Status::Pending => {
                // the voting hasn't even begun!
                error!("impossible case! the proposal {proposal_id} is still pending");
                Ok(ProposalResult::Pending)
            }
            Status::Open => {
                debug!("proposal {proposal_id} is still open and needs more votes");
                Ok(ProposalResult::Pending)
            }
            Status::Rejected => {
                warn!("proposal {proposal_id} has been rejected");
                Ok(ProposalResult::Rejected)
            }
            Status::Passed => {
                info!(
                    "proposal {proposal_id} has already been passed - we just need to execute it"
                );
                self.try_execute_proposal(proposal_id).await?;
                Ok(ProposalResult::Executed)
            }
            Status::Executed => {
                info!("proposal {proposal_id} has already been executed - nothing to do!");
                Ok(ProposalResult::Executed)
            }
        }
    }

    async fn try_resolve_pending_proposal(
        &self,
        pending: &mut PendingRedemptionVote,
        api_clients: Option<RwLockReadGuard<'_, Vec<EcashApiClient>>>,
    ) -> Result<ProposalResult, EcashTicketError> {
        let proposal_id = pending.proposal_id;

        info!(
            "attempting to resolve pending redemption proposal {proposal_id} to redeem {} tickets",
            pending.included_serial_numbers.len()
        );

        // check if the proposal still needs more votes from the apis
        let result = self.try_finalize_proposal(proposal_id).await?;
        if !result.is_pending() {
            return Ok(result);
        }

        let api_clients = match api_clients {
            Some(clients) => clients,
            None => self.shared_state.api_clients(pending.epoch_id).await?,
        };

        let redemption_request = pending.to_request_body(self.shared_state.address.clone());

        // TODO: optimisation: tell other apis they can purge our tickets even if they haven't voted

        let total = api_clients.len();
        let api_failures = Mutex::new(Vec::new());
        let rejected = AtomicUsize::new(0);

        for_each_api_concurrent(&api_clients, &pending.pending, |ecash_client| async {
            // errors are only returned on hard, storage, failures
            match self
                .redeem_tickets(pending.proposal_id, &redemption_request, ecash_client)
                .await
            {
                Err(err) => {
                    error!("internal failure. could not proceed with ticket redemption: {err}");
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
        if rejected_ratio >= (1. - MULTISIG_THRESHOLD) {
            error!("{rejected_ratio:.2} of signers rejected proposal {proposal_id}. we won't be able to execute it");
            // no need to query the chain as with so many rejections it's impossible it has passed.
            return Ok(ProposalResult::Rejected);
        }

        let accepted_ratio = (total - rejected - num_failures) as f32 / total as f32;
        match accepted_ratio {
            n if n < MULTISIG_THRESHOLD => {
                error!("less than 2/3 of signers ({n:.2}%) accepted proposal {proposal_id}. we're not yet be able to execute it to get funds out");
                return Ok(ProposalResult::Pending);
            }
            n if n < MINIMUM_API_QUORUM => {
                warn!("the system seems to be a bit unstable: less than 80%, but more than 67% of signers ({n:.2}%) accepted proposal {proposal_id}");
            }
            n => {
                trace!("{n:.2}% of signers accepted proposal {proposal_id}");
            }
        }

        // attempt to execute the proposal if it reached the required threshold
        self.try_finalize_proposal(proposal_id).await
    }

    async fn maybe_redeem_tickets(&mut self) -> Result<(), EcashTicketError> {
        if !self.pending_tickets.is_empty() {
            return Err(EcashTicketError::PendingTickets);
        }

        let latest_stored = self.shared_state.storage.latest_proposal().await?;

        // check if we have already created the proposal but crashed before persisting it in the db
        //
        // if we have some persisted proposals in storage, try to see if there's anything more recent on chain
        // (i.e. the missing proposal)
        // if not (i.e. this would have been our first) check the latest page of proposals.
        // while this is not ideal, realistically speaking we probably crashed few minutes ago
        // and worst case scenario we'll just recreate the proposal instead
        //
        // LIMITATION: if MULTIPLE proposals got created in between, well. though luck.
        let latest_on_chain = if let Some(latest_stored) = &latest_stored {
            // those are sorted in ASCENDING way
            self.shared_state
                .proposals_since(latest_stored.proposal_id as u64)
                .await?
                .pop()
        } else {
            // but those are DESCENDING
            self.shared_state
                .last_proposal_page()
                .await?
                .first()
                .cloned()
        };

        let now = OffsetDateTime::now_utc();

        let prior_proposal = match (latest_stored, latest_on_chain) {
            (None, None) => {
                // we haven't created any proposals before
                trace!("this could be our first redemption proposal");
                None
            }
            (Some(stored), None) => {
                if stored.created_at + MIN_BATCH_REDEMPTION_DELAY > now {
                    trace!("too soon to create new redemption proposal");
                    return Ok(());
                }
                None
            }
            (_, Some(on_chain)) => {
                warn!("we seem to have crashed after creating proposal, but before persisting it onto disk!");

                Some(on_chain)
            }
        };

        // technically we could have been just caching all of those serial numbers as we verify tickets,
        // but given how infrequently we call this, there's no point in wasting this memory
        let verified_tickets = self
            .shared_state
            .storage
            .get_all_verified_tickets_with_sn()
            .await?;

        if verified_tickets.len() < MINIMUM_REDEMPTION_TICKETS {
            info!("we only have {} verified tickets. there's no point in creating a redemption request yet.", verified_tickets.len());
            return Ok(());
        }

        // this should have been ensured when querying
        assert!(verified_tickets.len() <= u16::MAX as usize);

        let digest =
            BatchRedeemTicketsBody::make_digest(verified_tickets.iter().map(|t| &t.serial_number));
        let encoded_digest = bs58::encode(&digest).into_string();

        let prior_proposal_id = if let Some(prior_proposal) = prior_proposal {
            if prior_proposal.description == encoded_digest {
                info!("we have already created proposal for those tickets");
                Some(prior_proposal.id)
            } else {
                warn!(
                    "our missed proposal seem to have been for different tickets - abandoning it"
                );
                None
            }
        } else {
            None
        };

        // if the proposal has already existed on chain, do use it. otherwise create a new one
        let proposal_id = if let Some(prior) = prior_proposal_id {
            prior
        } else {
            self.create_redemption_proposal(&digest, verified_tickets.len() as u16)
                .await?
        };

        if proposal_id > u32::MAX as u64 {
            // realistically will we ever reach it? no.
            panic!(
                "we have created more than {} proposals. we can't handle that.",
                u32::MAX
            )
        }

        self.shared_state
            .storage
            .insert_redemption_proposal(
                &verified_tickets,
                proposal_id as u32,
                OffsetDateTime::now_utc(),
            )
            .await?;

        let current_epoch = self.shared_state.current_epoch_id().await?;
        let api_clients = self.shared_state.api_clients(current_epoch).await?;
        let ids = api_clients.iter().map(|c| c.node_id).collect();
        let mut pending = PendingRedemptionVote::new(
            proposal_id,
            digest,
            verified_tickets
                .into_iter()
                .map(|t| t.serial_number)
                .collect(),
            current_epoch,
            ids,
        );

        let resolution = self
            .try_resolve_pending_proposal(&mut pending, Some(api_clients))
            .await?;
        if resolution.is_pending() {
            warn!("failed to reach quorum for proposal {proposal_id}. apis: {:?} haven't responded. we'll retry later", pending.pending);
            self.pending_redemptions.push(pending);
        } else {
            self.shared_state
                .storage
                .clear_post_proposal_data(
                    proposal_id as u32,
                    OffsetDateTime::now_utc(),
                    resolution.is_rejected(),
                )
                .await?;
        }

        Ok(())
    }

    async fn periodic_operations(&mut self) -> Result<(), EcashTicketError> {
        trace!("attempting to resolve all pending operations -> tickets that are waiting for verification and possibly redemption");

        // 1. retry all operations that have failed in the past: verification requests and pending redemption
        self.resolve_pending().await?;

        // 2. if applicable, attempt to redeem all newly verified tickets
        self.maybe_redeem_tickets().await?;

        Ok(())
    }

    async fn run(mut self, mut shutdown: nym_task::TaskClient) {
        log::info!("Starting Ecash CredentialSender");

        // attempt to clear any pending operations
        info!("attempting to resolve any pending operations");
        if let Err(err) = self.periodic_operations().await {
            error!("failed to resolve pending operations on startup: {err}")
        }

        let start = Instant::now() + RESOLVER_INTERVAL;
        let mut resolver_interval = interval_at(start, RESOLVER_INTERVAL);

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("client_handling::credentialSender : received shutdown");
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

    pub(crate) fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
