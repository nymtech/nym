// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::controller::DkgController;
use crate::ecash::error::CoconutError;
use cw3::{ProposalResponse, Status};
use nym_coconut_dkg_common::verification_key::owner_from_cosmos_msgs;
use nym_validator_client::nyxd::AccountId;
use rand::{CryptoRng, RngCore};
use std::collections::HashMap;

impl<R: RngCore + CryptoRng> DkgController<R> {
    fn filter_proposal(
        &self,
        dkg_contract: &AccountId,
        proposal: &ProposalResponse,
    ) -> Option<(String, u64)> {
        // make sure the proposal we're checking is:
        // - still open (not point in voting for anything that has already expired)
        // - was proposed by the DKG contract - so that we'd ignore anything from malicious dealers
        // - contains valid verification request (checked inside `owner_from_cosmos_msgs`)
        if proposal.status == Status::Open && proposal.proposer.as_str() == dkg_contract.as_ref() {
            if let Some(owner) = owner_from_cosmos_msgs(&proposal.msgs) {
                return Some((owner, proposal.id));
            }
        }
        None
    }

    pub(crate) async fn get_validation_proposals(
        &self,
    ) -> Result<HashMap<String, u64>, CoconutError> {
        let dkg_contract = self.dkg_client.dkg_contract_address().await?;

        // FUTURE OPTIMIZATION: don't query for ALL proposals. say if we're in epoch 50,
        // we don't care about expired proposals from epochs 0-49...
        // to do it, we'll need to have dkg contract store proposal ids,
        // which will require usage of submsgs and replies so that might be a future project
        let all_proposals = self.dkg_client.list_proposals().await?;

        let mut deduped_proposals = HashMap::new();

        // for each proposal, make sure it's a valid validation request;
        // if for some reason there exist multiple proposals from the same owner, choose the one
        // with the higher id (there might be multiple since we're grabbing them across epochs)
        for proposal in all_proposals {
            if let Some((owner, id)) = self.filter_proposal(&dkg_contract, &proposal) {
                if let Some(old_id) = deduped_proposals.get(&owner) {
                    if old_id < &id {
                        deduped_proposals.insert(owner, id);
                    }
                } else {
                    deduped_proposals.insert(owner, id);
                }
            }
        }

        // UNHANDLED EDGE CASE:
        // since currently proposals are **NOT** tied to epochs,
        // we might run into proposals from older epochs we don't have to vote on or might not even have data for
        Ok(deduped_proposals)
    }
}
