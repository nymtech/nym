// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in tests
#![allow(clippy::unreachable)]

//! In-process fake ecash signer implementing `NymApiClientExt` directly, so the issuance audit
//! runs against real ecash partial signatures with no HTTP transport. Mocked at the trait
//! boundary like `MockNymApiClient` in `nym-directory-client`: the audit methods are overridden
//! with canned, signed responses and the lower-level `ApiClientCore` hooks are never reached.

use crate::rewarder::ticketbook_issuance::types::CredentialIssuer;
use async_trait::async_trait;
use nym_compact_ecash::scheme::keygen::{KeyPairAuth, SecretKeyAuth};
use nym_compact_ecash::{Bytable, generate_keypair_user, issue, ttp_keygen, withdrawal_request};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::EcashTime;
use nym_http_api_client::reqwest::{Method, RequestBuilder, Response, Url as ClientUrl};
use nym_http_api_client::{ApiClientCore, HttpClientError, Params, RequestPath, Url as CoreUrl};
use nym_ticketbooks_merkle::{IssuedTicketbook, IssuedTicketbooksMerkleTree, MerkleLeaf};
use nym_validator_client::ecash::models::{
    CommitedDeposit, DepositId, IssuedTicketbooksChallengeCommitmentRequest,
    IssuedTicketbooksChallengeCommitmentResponse, IssuedTicketbooksChallengeCommitmentResponseBody,
    IssuedTicketbooksDataRequest, IssuedTicketbooksDataResponse, IssuedTicketbooksDataResponseBody,
    IssuedTicketbooksForCountResponse, IssuedTicketbooksForResponse,
    IssuedTicketbooksForResponseBody,
};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nym_api::error::NymAPIError;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::signable::SignableMessageBody;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use time::Date;

pub(crate) const MAX_DATA_RESPONSE_SIZE: usize = 100;

/// How the fake signer deviates from an honest nym-api.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Misbehaviour {
    None,
    /// commitment lists deposits but carries no merkle root
    NoMerkleRoot,
    /// commitment reports the day after the requested date
    WrongExpirationDate,
    /// challenge response echoes an altered original request
    TamperedEcho,
    /// every data batch omits its last ticketbook
    ShortDataBatch,
}

#[derive(Default)]
struct Cohort {
    tree: IssuedTicketbooksMerkleTree,
    leaves: HashMap<DepositId, MerkleLeaf>,
    ticketbooks: BTreeMap<DepositId, IssuedTicketbook>,
}

struct FakeSignerInner {
    identity: ed25519::KeyPair,
    ecash_keys: KeyPairAuth,
    operator_account: AccountId,
    epoch_id: u64,
    misbehaviour: Misbehaviour,
    // never dialled - the audit methods are overridden
    url: ClientUrl,
    cohorts: Mutex<HashMap<Date, Cohort>>,
    // leaf indices derived from the most recent challenge request, in the order received
    challenge_indices: Mutex<Vec<usize>>,
}

/// A cloneable handle to an in-process fake ecash signer.
#[derive(Clone)]
pub(crate) struct FakeSigner {
    inner: Arc<FakeSignerInner>,
}

impl FakeSigner {
    pub(crate) fn new(operator_account_byte: u8, misbehaviour: Misbehaviour) -> Self {
        FakeSigner {
            inner: Arc::new(FakeSignerInner {
                identity: nym_test_utils::helpers::dummy_ed25519_keypair(
                    operator_account_byte as u64,
                ),
                ecash_keys: ttp_keygen(1, 1).unwrap().remove(0),
                operator_account: AccountId::new("n", &[operator_account_byte; 20]).unwrap(),
                epoch_id: 0,
                misbehaviour,
                url: "http://mock.invalid".parse().unwrap(),
                cohorts: Mutex::new(HashMap::new()),
                challenge_indices: Mutex::new(Vec::new()),
            }),
        }
    }

    pub(crate) fn operator_account(&self) -> AccountId {
        self.inner.operator_account.clone()
    }

    /// The leaf indices the signer derived from the most recent challenge request, in request order.
    pub(crate) fn last_challenge_indices(&self) -> Vec<usize> {
        self.inner.challenge_indices.lock().unwrap().clone()
    }

    /// Issues one real partial ticketbook signed with this signer's advertised key.
    pub(crate) fn issue_ticketbook(&self, deposit_id: DepositId, expiration_date: Date) {
        self.issue_with_key(
            deposit_id,
            expiration_date,
            self.inner.ecash_keys.secret_key(),
        )
    }

    /// Issues a ticketbook under a key never advertised: consistent leaf and proof, invalid signature.
    pub(crate) fn issue_foreign_key_ticketbook(
        &self,
        deposit_id: DepositId,
        expiration_date: Date,
    ) {
        let foreign = ttp_keygen(1, 1).unwrap().remove(0);
        self.issue_with_key(deposit_id, expiration_date, foreign.secret_key())
    }

    fn issue_with_key(&self, deposit_id: DepositId, expiration_date: Date, sk: &SecretKeyAuth) {
        let ticket_type = TicketType::V1MixnetEntry;
        let exp_ts = expiration_date.ecash_unix_timestamp();
        let user = generate_keypair_user();
        let (request, _) =
            withdrawal_request(user.secret_key(), exp_ts, ticket_type.encode()).unwrap();
        let blinded = issue(
            sk,
            user.public_key(),
            &request,
            exp_ts,
            ticket_type.encode(),
        )
        .unwrap();
        let issued = IssuedTicketbook {
            deposit_id,
            epoch_id: self.inner.epoch_id,
            blinded_partial_credential: blinded.to_byte_vec(),
            joined_encoded_private_attributes_commitments: request
                .get_private_attributes_commitments()
                .iter()
                .flat_map(|c| c.to_byte_vec())
                .collect(),
            expiration_date,
            ticketbook_type: ticket_type,
        };

        let mut cohorts = self.inner.cohorts.lock().unwrap();
        let cohort = cohorts.entry(expiration_date).or_default();
        let inserted = cohort.tree.insert(&issued);
        cohort.leaves.insert(deposit_id, inserted.leaf);
        cohort.ticketbooks.insert(deposit_id, issued);
    }

    /// The issuer record the rewarder would have built from the DKG contract.
    pub(crate) fn as_credential_issuer(&self, node_id: u64) -> CredentialIssuer<FakeSigner> {
        CredentialIssuer {
            public_key: *self.inner.identity.public_key(),
            operator_account: self.inner.operator_account.clone(),
            verification_key: self.inner.ecash_keys.verification_key(),
            node_id,
            api_client: self.clone(),
        }
    }
}

#[async_trait]
impl ApiClientCore for FakeSigner {
    fn create_request<P, B, K, V>(
        &self,
        _method: Method,
        _path: P,
        _params: Params<'_, K, V>,
        _body: Option<&B>,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        unreachable!("the mock overrides the audit methods, so no request is ever built")
    }

    async fn send(&self, _request: RequestBuilder) -> Result<Response, HttpClientError> {
        unreachable!("the mock overrides the audit methods, so no request is ever sent")
    }

    fn maybe_rotate_hosts(&self, _offending_url: Option<CoreUrl>) {}

    fn maybe_enable_fronting(&self, _context: impl std::fmt::Debug) {}
}

#[async_trait]
impl NymApiClientExt for FakeSigner {
    fn api_url(&self) -> &ClientUrl {
        &self.inner.url
    }

    fn change_base_urls(&mut self, _urls: Vec<ClientUrl>) {}

    async fn issued_ticketbooks_for_count(
        &self,
        expiration_date: Date,
    ) -> Result<IssuedTicketbooksForCountResponse, NymAPIError> {
        let total = self
            .inner
            .cohorts
            .lock()
            .unwrap()
            .get(&expiration_date)
            .map(|c| c.ticketbooks.len())
            .unwrap_or(0);
        Ok(IssuedTicketbooksForCountResponse {
            expiration_date,
            total,
            issued: vec![],
        })
    }

    async fn issued_ticketbooks_for(
        &self,
        expiration_date: Date,
    ) -> Result<IssuedTicketbooksForResponse, NymAPIError> {
        let reported = match self.inner.misbehaviour {
            Misbehaviour::WrongExpirationDate => expiration_date.next_day().unwrap(),
            _ => expiration_date,
        };

        let (deposits, merkle_root): (Vec<CommitedDeposit>, Option<[u8; 32]>) = {
            let cohorts = self.inner.cohorts.lock().unwrap();
            match cohorts.get(&expiration_date) {
                Some(c) => (
                    c.leaves
                        .iter()
                        .map(|(&deposit_id, leaf)| CommitedDeposit {
                            deposit_id,
                            merkle_index: leaf.index,
                        })
                        .collect(),
                    c.tree.root(),
                ),
                None => (vec![], None),
            }
        };
        let merkle_root = match self.inner.misbehaviour {
            Misbehaviour::NoMerkleRoot => None,
            _ => merkle_root,
        };

        Ok(IssuedTicketbooksForResponseBody {
            expiration_date: reported,
            deposits,
            merkle_root,
        }
        .sign(self.inner.identity.private_key()))
    }

    async fn issued_ticketbooks_challenge_commitment(
        &self,
        request: &IssuedTicketbooksChallengeCommitmentRequest,
    ) -> Result<IssuedTicketbooksChallengeCommitmentResponse, NymAPIError> {
        let expiration_date = request.body.expiration_date;
        let merkle_proof = {
            let cohorts = self.inner.cohorts.lock().unwrap();
            let cohort = cohorts
                .get(&expiration_date)
                .expect("challenge for a cohort the fake signer never issued");
            let indices: Vec<usize> = request
                .body
                .deposits
                .iter()
                .map(|d| cohort.leaves[d].index)
                .collect();
            *self.inner.challenge_indices.lock().unwrap() = indices.clone();
            cohort
                .tree
                .generate_proof(&indices)
                .expect("proof generation")
        };

        let mut request = request.clone();
        if self.inner.misbehaviour == Misbehaviour::TamperedEcho {
            request.body.deposits.push(u32::MAX);
        }

        Ok(IssuedTicketbooksChallengeCommitmentResponseBody {
            expiration_date,
            original_request: request,
            max_data_response_size: MAX_DATA_RESPONSE_SIZE,
            merkle_proof,
        }
        .sign(self.inner.identity.private_key()))
    }

    async fn issued_ticketbooks_data(
        &self,
        request: &IssuedTicketbooksDataRequest,
    ) -> Result<IssuedTicketbooksDataResponse, NymAPIError> {
        let expiration_date = request.body.expiration_date;
        let mut partial_ticketbooks: BTreeMap<DepositId, IssuedTicketbook> = {
            let cohorts = self.inner.cohorts.lock().unwrap();
            let cohort = cohorts
                .get(&expiration_date)
                .expect("data request for a cohort the fake signer never issued");
            request
                .body
                .deposits
                .iter()
                .map(|d| (*d, cohort.ticketbooks[d].clone()))
                .collect()
        };

        if self.inner.misbehaviour == Misbehaviour::ShortDataBatch {
            partial_ticketbooks.pop_last();
        }

        Ok(IssuedTicketbooksDataResponseBody {
            expiration_date,
            partial_ticketbooks,
            original_request: request.clone(),
        }
        .sign(self.inner.identity.private_key()))
    }
}
