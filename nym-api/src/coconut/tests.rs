// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::InternalSignRequest;
use crate::coconut::error::{CoconutError, Result};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, WasmMsg};
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, VerifyCredentialBody, VerifyCredentialResponse,
};
use nym_coconut::tests::helpers::theta_from_keys_and_attributes;
use nym_coconut::{prepare_blind_sign, ttp_keygen, Base58, BlindedSignature, Parameters};
use nym_coconut_bandwidth_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_IDENTITY_KEY, DEPOSIT_INFO,
    DEPOSIT_VALUE,
};
use nym_coconut_bandwidth_contract_common::spend_credential::{
    SpendCredential, SpendCredentialResponse,
};
use nym_coconut_interface::{hash_to_scalar, Credential, VerificationKey};
use nym_config::defaults::VOUCHER_INFO;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use nym_credentials::coconut::params::{
    NymApiCredentialEncryptionAlgorithm, NymApiCredentialHkdfAlgorithm,
};
use nym_crypto::shared_key::recompute_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_validator_client::nym_api::routes::{
    API_VERSION, BANDWIDTH, COCONUT_BLIND_SIGN, COCONUT_ROUTES, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL,
};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::{
    AccountId, Algorithm, Event, EventAttribute, ExecTxResult, Fee, Hash, TxResponse,
};

use crate::coconut::State;
use crate::support::storage::NymApiStorage;
use async_trait::async_trait;
use cw3::ProposalResponse;
use cw4::MemberResponse;
use nym_coconut_dkg_common::dealer::{
    ContractDealing, DealerDetails, DealerDetailsResponse, DealerType,
};
use nym_coconut_dkg_common::event_attributes::{DKG_PROPOSAL_ID, NODE_INDEX};
use nym_coconut_dkg_common::types::{
    EncodedBTEPublicKeyWithProof, Epoch, EpochId, InitialReplacementData, TOTAL_DEALINGS,
};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use nym_contracts_common::dealings::ContractSafeBytes;
use nym_crypto::asymmetric::{encryption, identity};
use nym_dkg::Threshold;
use nym_validator_client::nyxd::cosmwasm_client::logs::Log;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use rand_07::rngs::OsRng;
use rand_07::Rng;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

const TEST_COIN_DENOM: &str = "unym";
const TEST_REWARDING_VALIDATOR_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";

#[derive(Clone, Debug)]
pub(crate) struct DummyClient {
    validator_address: AccountId,
    tx_db: Arc<RwLock<HashMap<String, TxResponse>>>,
    proposal_db: Arc<RwLock<HashMap<u64, ProposalResponse>>>,
    spent_credential_db: Arc<RwLock<HashMap<String, SpendCredentialResponse>>>,

    epoch: Arc<RwLock<Epoch>>,
    dealer_details: Arc<RwLock<HashMap<String, (DealerDetails, bool)>>>,
    threshold: Arc<RwLock<Option<Threshold>>>,
    dealings: Arc<RwLock<HashMap<String, Vec<ContractSafeBytes>>>>,
    verification_share: Arc<RwLock<HashMap<String, ContractVKShare>>>,
    group_db: Arc<RwLock<HashMap<String, MemberResponse>>>,
    initial_dealers_db: Arc<RwLock<Option<InitialReplacementData>>>,
}

impl DummyClient {
    pub fn new(validator_address: AccountId) -> Self {
        Self {
            validator_address,
            tx_db: Arc::new(RwLock::new(HashMap::new())),
            proposal_db: Arc::new(RwLock::new(HashMap::new())),
            spent_credential_db: Arc::new(RwLock::new(HashMap::new())),
            epoch: Arc::new(RwLock::new(Epoch::default())),
            dealer_details: Arc::new(RwLock::new(HashMap::new())),
            threshold: Arc::new(RwLock::new(None)),
            dealings: Arc::new(RwLock::new(HashMap::new())),
            verification_share: Arc::new(RwLock::new(HashMap::new())),
            group_db: Arc::new(RwLock::new(HashMap::new())),
            initial_dealers_db: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_tx_db(mut self, tx_db: &Arc<RwLock<HashMap<String, TxResponse>>>) -> Self {
        self.tx_db = Arc::clone(tx_db);
        self
    }

    pub fn with_proposal_db(
        mut self,
        proposal_db: &Arc<RwLock<HashMap<u64, ProposalResponse>>>,
    ) -> Self {
        self.proposal_db = Arc::clone(proposal_db);
        self
    }

    pub fn with_spent_credential_db(
        mut self,
        spent_credential_db: &Arc<RwLock<HashMap<String, SpendCredentialResponse>>>,
    ) -> Self {
        self.spent_credential_db = Arc::clone(spent_credential_db);
        self
    }

    pub fn _with_epoch(mut self, epoch: &Arc<RwLock<Epoch>>) -> Self {
        self.epoch = Arc::clone(epoch);
        self
    }

    pub fn with_dealer_details(
        mut self,
        dealer_details: &Arc<RwLock<HashMap<String, (DealerDetails, bool)>>>,
    ) -> Self {
        self.dealer_details = Arc::clone(dealer_details);
        self
    }

    pub fn with_threshold(mut self, threshold: &Arc<RwLock<Option<Threshold>>>) -> Self {
        self.threshold = Arc::clone(threshold);
        self
    }

    pub fn with_dealings(
        mut self,
        dealings: &Arc<RwLock<HashMap<String, Vec<ContractSafeBytes>>>>,
    ) -> Self {
        self.dealings = Arc::clone(dealings);
        self
    }

    pub fn with_verification_share(
        mut self,
        verification_share: &Arc<RwLock<HashMap<String, ContractVKShare>>>,
    ) -> Self {
        self.verification_share = Arc::clone(verification_share);
        self
    }

    pub fn _with_group_db(
        mut self,
        group_db: &Arc<RwLock<HashMap<String, MemberResponse>>>,
    ) -> Self {
        self.group_db = Arc::clone(group_db);
        self
    }

    pub fn with_initial_dealers_db(
        mut self,
        initial_dealers: &Arc<RwLock<Option<InitialReplacementData>>>,
    ) -> Self {
        self.initial_dealers_db = Arc::clone(initial_dealers);
        self
    }
}

#[async_trait]
impl super::client::Client for DummyClient {
    async fn address(&self) -> AccountId {
        self.validator_address.clone()
    }

    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse> {
        self.tx_db
            .read()
            .unwrap()
            .get(tx_hash)
            .cloned()
            .ok_or(CoconutError::TxHashParseError)
    }

    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse> {
        self.proposal_db
            .read()
            .unwrap()
            .get(&proposal_id)
            .cloned()
            .ok_or(CoconutError::IncorrectProposal {
                reason: String::from("proposal not found"),
            })
    }

    async fn list_proposals(&self) -> Result<Vec<ProposalResponse>> {
        Ok(self.proposal_db.read().unwrap().values().cloned().collect())
    }

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse> {
        self.spent_credential_db
            .read()
            .unwrap()
            .get(&blinded_serial_number)
            .cloned()
            .ok_or(CoconutError::InvalidCredentialStatus {
                status: String::from("spent credential not found"),
            })
    }

    async fn get_current_epoch(&self) -> Result<Epoch> {
        Ok(*self.epoch.read().unwrap())
    }

    async fn group_member(&self, addr: String) -> Result<MemberResponse> {
        Ok(self
            .group_db
            .read()
            .unwrap()
            .get(&addr)
            .cloned()
            .unwrap_or(MemberResponse { weight: None }))
    }

    async fn get_current_epoch_threshold(&self) -> Result<Option<Threshold>> {
        Ok(*self.threshold.read().unwrap())
    }

    async fn get_initial_dealers(&self) -> Result<Option<InitialReplacementData>> {
        Ok(self.initial_dealers_db.read().unwrap().clone())
    }

    async fn get_self_registered_dealer_details(&self) -> Result<DealerDetailsResponse> {
        let (details, dealer_type) = if let Some((details, current)) = self
            .dealer_details
            .read()
            .unwrap()
            .get(self.validator_address.as_ref())
            .cloned()
        {
            let dealer_type = if current {
                DealerType::Current
            } else {
                DealerType::Past
            };
            (Some(details), dealer_type)
        } else {
            (None, DealerType::Unknown)
        };
        Ok(DealerDetailsResponse {
            details,
            dealer_type,
        })
    }

    async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>> {
        Ok(self
            .dealer_details
            .read()
            .unwrap()
            .values()
            .cloned()
            .filter_map(|(d, current)| if current { Some(d) } else { None })
            .collect())
    }

    async fn get_dealings(&self, idx: usize) -> Result<Vec<ContractDealing>> {
        Ok(self
            .dealings
            .read()
            .unwrap()
            .iter()
            .map(|(dealer, dealings)| ContractDealing {
                dealing: dealings.get(idx).unwrap().clone(),
                dealer: Addr::unchecked(dealer),
            })
            .collect())
    }

    async fn get_verification_key_shares(
        &self,
        _epoch_id: EpochId,
    ) -> Result<Vec<ContractVKShare>> {
        Ok(self
            .verification_share
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect())
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        _fee: Option<Fee>,
    ) -> Result<()> {
        if let Some(proposal) = self.proposal_db.write().unwrap().get_mut(&proposal_id) {
            // for now, just suppose that every vote is honest
            if !vote_yes {
                proposal.status = cw3::Status::Rejected;
            } else if vote_yes && proposal.status == cw3::Status::Open {
                proposal.status = cw3::Status::Passed;
            }
        }
        Ok(())
    }

    async fn execute_proposal(&self, proposal_id: u64) -> Result<()> {
        self.proposal_db
            .write()
            .unwrap()
            .entry(proposal_id)
            .and_modify(|prop| {
                if prop.status == cw3::Status::Passed {
                    prop.status = cw3::Status::Executed
                }
            });
        Ok(())
    }

    async fn advance_epoch_state(&self) -> Result<()> {
        todo!()
    }

    async fn register_dealer(
        &self,
        bte_public_key_with_proof: EncodedBTEPublicKeyWithProof,
        announce_address: String,
        _resharing: bool,
    ) -> Result<ExecuteResult> {
        let mut dealer_details = self.dealer_details.write().unwrap();
        let assigned_index = if let Some((details, active)) =
            dealer_details.get_mut(self.validator_address.as_ref())
        {
            *active = true;
            details.assigned_index
        } else {
            // let assigned_index = OsRng.gen();
            let assigned_index = dealer_details
                .values()
                .map(|(d, _)| d.assigned_index)
                .max()
                .unwrap_or(0)
                + 1;
            dealer_details.insert(
                self.validator_address.to_string(),
                (
                    DealerDetails {
                        address: Addr::unchecked(self.validator_address.to_string()),
                        bte_public_key_with_proof,
                        announce_address,
                        assigned_index,
                    },
                    true,
                ),
            );
            assigned_index
        };
        Ok(ExecuteResult {
            logs: vec![Log {
                msg_index: 0,
                events: vec![cosmwasm_std::Event::new("wasm")
                    .add_attribute(NODE_INDEX, assigned_index.to_string())],
            }],
            data: Default::default(),
            transaction_hash: Hash::from_bytes(Algorithm::Sha256, &[0; 32]).unwrap(),
            gas_info: Default::default(),
        })
    }

    async fn submit_dealing(
        &self,
        dealing_bytes: ContractSafeBytes,
        _resharing: bool,
    ) -> Result<ExecuteResult> {
        self.dealings
            .write()
            .unwrap()
            .entry(self.validator_address.to_string())
            .and_modify(|v| {
                if v.len() < TOTAL_DEALINGS {
                    v.push(dealing_bytes.clone())
                }
            })
            .or_insert_with(|| vec![dealing_bytes]);

        Ok(ExecuteResult {
            logs: vec![],
            data: Default::default(),
            transaction_hash: Hash::from_bytes(Algorithm::Sha256, &[0; 32]).unwrap(),
            gas_info: Default::default(),
        })
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> Result<ExecuteResult> {
        let (dealer_details, active) = self
            .dealer_details
            .read()
            .unwrap()
            .get(self.validator_address.as_ref())
            .unwrap()
            .clone();
        if !active {
            // Just throw some error, not really the correct one
            return Err(CoconutError::DepositEncrKeyNotFound);
        }
        self.verification_share.write().unwrap().insert(
            self.validator_address.to_string(),
            ContractVKShare {
                share,
                announce_address: dealer_details.announce_address.clone(),
                node_index: dealer_details.assigned_index,
                owner: Addr::unchecked(self.validator_address.to_string()),
                epoch_id: 0,
                verified: false,
            },
        );
        let proposal_id = OsRng.gen();
        let verify_vk_share_req =
            nym_coconut_dkg_common::msg::ExecuteMsg::VerifyVerificationKeyShare {
                owner: Addr::unchecked(self.validator_address.as_ref()),
                resharing,
            };
        let verify_vk_share_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::new(),
            msg: to_binary(&verify_vk_share_req).unwrap(),
            funds: vec![],
        });
        let proposal = ProposalResponse {
            id: proposal_id,
            title: String::new(),
            description: String::new(),
            msgs: vec![verify_vk_share_msg],
            status: cw3::Status::Open,
            expires: cw_utils::Expiration::Never {},
            threshold: cw_utils::ThresholdResponse::AbsolutePercentage {
                percentage: Decimal::from_ratio(2u32, 3u32),
                total_weight: 100,
            },
            proposer: Addr::unchecked(self.validator_address.as_ref()),
            deposit: None,
        };
        self.proposal_db
            .write()
            .unwrap()
            .insert(proposal_id, proposal);
        Ok(ExecuteResult {
            logs: vec![Log {
                msg_index: 0,
                events: vec![cosmwasm_std::Event::new("wasm")
                    .add_attribute(DKG_PROPOSAL_ID, proposal_id.to_string())],
            }],
            data: Default::default(),
            transaction_hash: Hash::from_bytes(Algorithm::Sha256, &[0; 32]).unwrap(),
            gas_info: Default::default(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct DummyCommunicationChannel {
    aggregated_verification_key: VerificationKey,
}

impl DummyCommunicationChannel {
    pub fn new(aggregated_verification_key: VerificationKey) -> Self {
        DummyCommunicationChannel {
            aggregated_verification_key,
        }
    }
}

#[async_trait]
impl super::comm::APICommunicationChannel for DummyCommunicationChannel {
    async fn aggregated_verification_key(&self, _epoch_id: EpochId) -> Result<VerificationKey> {
        Ok(self.aggregated_verification_key.clone())
    }
}

pub fn tx_entry_fixture(tx_hash: &str) -> TxResponse {
    TxResponse {
        hash: Hash::from_str(tx_hash).unwrap(),
        height: Default::default(),
        index: 0,
        tx_result: ExecTxResult {
            code: Default::default(),
            data: Default::default(),
            log: Default::default(),
            info: Default::default(),
            gas_wanted: Default::default(),
            gas_used: Default::default(),
            events: vec![],
            codespace: Default::default(),
        },
        tx: vec![],
        proof: None,
    }
}

#[tokio::test]
async fn signed_before() {
    let tx_hash =
        Hash::from_str("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E").unwrap();
    let tx_entry = tx_entry_fixture(&tx_hash.to_string());
    let signature = String::from(
        "2DHbEZ6pzToGpsAXJrqJi7Wj1pAXeT18283q2YEEyNH5gTymwRozWBdja6SMAVt1dyYmUnM4ZNhsJ4wxZyGh4Z6J",
    );

    let params = Parameters::new(4).unwrap();
    let mut rng = OsRng;
    let voucher = BandwidthVoucher::new(
        &params,
        "1234".to_string(),
        VOUCHER_INFO.to_string(),
        tx_hash,
        identity::PrivateKey::from_base58_string(
            identity::KeyPair::new(&mut rng)
                .private_key()
                .to_base58_string(),
        )
        .unwrap(),
        encryption::PrivateKey::from_bytes(
            &encryption::KeyPair::new(&mut rng).private_key().to_bytes(),
        )
        .unwrap(),
    );
    let (_, blind_sign_req) = prepare_blind_sign(
        &params,
        &voucher.get_private_attributes(),
        &voucher.get_public_attributes(),
    )
    .unwrap();

    let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
    let mut db_dir = std::env::temp_dir();
    db_dir.push(&key_pair.verification_key().to_bs58()[..8]);
    let storage = NymApiStorage::init(db_dir).await.unwrap();
    let tx_db = Arc::new(RwLock::new(HashMap::new()));
    tx_db
        .write()
        .unwrap()
        .insert(tx_hash.to_string(), tx_entry.clone());
    let nyxd_client =
        DummyClient::new(AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap())
            .with_tx_db(&tx_db);
    let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key());
    let staged_key_pair = crate::coconut::KeyPair::new();
    staged_key_pair.set(Some(key_pair)).await;

    let rocket = rocket::build().attach(InternalSignRequest::stage(
        nyxd_client,
        TEST_COIN_DENOM.to_string(),
        staged_key_pair,
        comm_channel,
        storage.clone(),
    ));
    let client = Client::tracked(rocket)
        .await
        .expect("valid rocket instance");

    let request_body = BlindSignRequestBody::new(
        &blind_sign_req,
        tx_hash.to_string(),
        signature.clone(),
        &voucher.get_public_attributes(),
        voucher.get_public_attributes_plain(),
        4,
    );

    let encrypted_signature = vec![1, 2, 3, 4];
    let remote_key = [42; 32];
    let expected_response = BlindedSignatureResponse::new(encrypted_signature, remote_key);
    storage
        .insert_blinded_signature_response(
            &tx_hash.to_string(),
            &expected_response.to_base58_string(),
        )
        .await
        .unwrap();

    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_BLIND_SIGN
        ))
        .json(&request_body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // This is a more direct way, but there's a bug which makes it hang https://github.com/SergioBenitez/Rocket/issues/1893
    // let blinded_signature_response = response
    //     .into_json::<BlindedSignatureResponse>()
    //     .await
    //     .unwrap();
    let blinded_signature_response =
        serde_json::from_str::<BlindedSignatureResponse>(&response.into_string().await.unwrap())
            .unwrap();
    assert_eq!(
        blinded_signature_response.to_bytes(),
        expected_response.to_bytes()
    );
}

#[tokio::test]
async fn state_functions() {
    let nyxd_client =
        DummyClient::new(AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap());
    let params = Parameters::new(4).unwrap();
    let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
    let mut db_dir = std::env::temp_dir();
    db_dir.push(&key_pair.verification_key().to_bs58()[..8]);
    let storage = NymApiStorage::init(db_dir).await.unwrap();
    let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key());
    let staged_key_pair = crate::coconut::KeyPair::new();
    staged_key_pair.set(Some(key_pair)).await;
    let state = State::new(
        nyxd_client,
        TEST_COIN_DENOM.to_string(),
        staged_key_pair,
        comm_channel,
        storage.clone(),
    );

    let tx_hash = String::from("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E");
    assert!(state.signed_before(&tx_hash).await.unwrap().is_none());

    let encrypted_signature = vec![1, 2, 3, 4];
    let remote_key = [42; 32];
    let expected_response = BlindedSignatureResponse::new(encrypted_signature, remote_key);
    storage
        .insert_blinded_signature_response(&tx_hash, &expected_response.to_base58_string())
        .await
        .unwrap();
    assert_eq!(
        state
            .signed_before(&tx_hash)
            .await
            .unwrap()
            .unwrap()
            .to_bytes(),
        expected_response.to_bytes()
    );

    let encryption_keypair = nym_crypto::asymmetric::encryption::KeyPair::new(&mut OsRng);
    let blinded_signature = BlindedSignature::from_bytes(&[
        183, 217, 166, 113, 40, 123, 74, 25, 72, 31, 136, 19, 125, 95, 217, 228, 96, 113, 25, 240,
        12, 102, 125, 11, 174, 20, 216, 82, 192, 71, 27, 194, 48, 20, 17, 95, 243, 179, 82, 21, 57,
        143, 101, 19, 22, 186, 147, 13, 147, 238, 39, 119, 15, 36, 251, 131, 250, 38, 185, 113,
        187, 40, 227, 107, 134, 190, 123, 183, 126, 176, 226, 173, 147, 137, 17, 175, 13, 115, 78,
        222, 119, 93, 146, 116, 229, 0, 152, 51, 232, 2, 102, 204, 147, 202, 254, 243,
    ])
    .unwrap();
    // Check that the new payload is not stored if there was already something signed for tx_hash
    assert_eq!(
        state
            .encrypt_and_store(
                &tx_hash,
                encryption_keypair.public_key(),
                &blinded_signature,
            )
            .await
            .unwrap()
            .to_bytes(),
        expected_response.to_bytes()
    );

    // And use a new hash to store a new signature
    let tx_hash = String::from("97D64C38D6601B1F0FD3A82E20D252685CB7A210AFB0261018590659AB82B0BF");
    let response = state
        .encrypt_and_store(
            &tx_hash,
            encryption_keypair.public_key(),
            &blinded_signature,
        )
        .await
        .unwrap();
    let remote_key =
        nym_crypto::asymmetric::encryption::PublicKey::from_bytes(&response.remote_key).unwrap();

    let encryption_key = recompute_shared_key::<
        NymApiCredentialEncryptionAlgorithm,
        NymApiCredentialHkdfAlgorithm,
    >(&remote_key, encryption_keypair.private_key());
    let zero_iv = stream_cipher::zero_iv::<NymApiCredentialEncryptionAlgorithm>();
    let blinded_signature_bytes = stream_cipher::decrypt::<NymApiCredentialEncryptionAlgorithm>(
        &encryption_key,
        &zero_iv,
        &response.encrypted_signature,
    );

    let received_blinded_signature =
        BlindedSignature::from_bytes(&blinded_signature_bytes).unwrap();
    assert_eq!(
        blinded_signature.to_bytes(),
        received_blinded_signature.to_bytes()
    );

    // Check that the same value for tx_hash is returned

    let other_signature = BlindedSignature::from_bytes(&[
        183, 217, 166, 113, 40, 123, 74, 25, 72, 31, 136, 19, 125, 95, 217, 228, 96, 113, 25, 240,
        12, 102, 125, 11, 174, 20, 216, 82, 192, 71, 27, 194, 48, 20, 17, 95, 243, 179, 82, 21, 57,
        143, 101, 19, 22, 186, 147, 13, 131, 236, 38, 138, 192, 235, 169, 142, 176, 118, 153, 238,
        141, 91, 94, 139, 168, 214, 17, 250, 96, 206, 139, 89, 139, 87, 31, 8, 106, 171, 8, 140,
        201, 158, 18, 152, 24, 98, 153, 170, 141, 35, 190, 200, 19, 148, 71, 197,
    ])
    .unwrap();
    assert_eq!(
        state
            .encrypt_and_store(&tx_hash, encryption_keypair.public_key(), &other_signature,)
            .await
            .unwrap()
            .to_bytes(),
        response.to_bytes()
    );
}

#[tokio::test]
async fn blind_sign_correct() {
    let tx_hash =
        Hash::from_str("7C41AF8266D91DE55E1C8F4712E6A952A165ED3D8C27C7B00428CBD0DE00A52B").unwrap();

    let params = Parameters::new(4).unwrap();
    let mut rng = OsRng;
    let identity_keypair = identity::KeyPair::new(&mut rng);
    let encryption_keypair = encryption::KeyPair::new(&mut rng);
    let voucher = BandwidthVoucher::new(
        &params,
        "1234".to_string(),
        VOUCHER_INFO.to_string(),
        tx_hash,
        identity::PrivateKey::from_base58_string(identity_keypair.private_key().to_base58_string())
            .unwrap(),
        encryption::PrivateKey::from_bytes(&encryption_keypair.private_key().to_bytes()).unwrap(),
    );

    let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
    let mut db_dir = std::env::temp_dir();
    db_dir.push(&key_pair.verification_key().to_bs58()[..8]);
    let storage = NymApiStorage::init(db_dir).await.unwrap();
    let tx_db = Arc::new(RwLock::new(HashMap::new()));

    let mut tx_entry = tx_entry_fixture(&tx_hash.to_string());
    tx_entry.tx_result.events.push(Event {
        kind: format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE),
        attributes: vec![],
    });
    tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
        EventAttribute {
            key: DEPOSIT_VALUE.parse().unwrap(),
            value: "1234".parse().unwrap(),
            index: false,
        },
        EventAttribute {
            key: DEPOSIT_INFO.parse().unwrap(),
            value: VOUCHER_INFO.parse().unwrap(),
            index: false,
        },
        EventAttribute {
            key: DEPOSIT_IDENTITY_KEY.parse().unwrap(),
            value: identity_keypair
                .public_key()
                .to_base58_string()
                .parse()
                .unwrap(),
            index: false,
        },
        EventAttribute {
            key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
            value: encryption_keypair
                .public_key()
                .to_base58_string()
                .parse()
                .unwrap(),
            index: false,
        },
    ];
    tx_db
        .write()
        .unwrap()
        .insert(tx_hash.to_string(), tx_entry.clone());
    let nyxd_client =
        DummyClient::new(AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap())
            .with_tx_db(&tx_db);
    let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key());
    let staged_key_pair = crate::coconut::KeyPair::new();
    staged_key_pair.set(Some(key_pair)).await;

    let rocket = rocket::build().attach(InternalSignRequest::stage(
        nyxd_client,
        TEST_COIN_DENOM.to_string(),
        staged_key_pair,
        comm_channel,
        storage.clone(),
    ));
    let client = Client::tracked(rocket)
        .await
        .expect("valid rocket instance");

    let request_body = BlindSignRequestBody::new(
        voucher.blind_sign_request(),
        tx_hash.to_string(),
        voucher
            .sign(voucher.blind_sign_request())
            .to_base58_string(),
        &voucher.get_public_attributes(),
        voucher.get_public_attributes_plain(),
        4,
    );

    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_BLIND_SIGN
        ))
        .json(&request_body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    // This is a more direct way, but there's a bug which makes it hang https://github.com/SergioBenitez/Rocket/issues/1893
    // assert!(response.into_json::<BlindedSignatureResponse>().is_some());
    let blinded_signature_response =
        serde_json::from_str::<BlindedSignatureResponse>(&response.into_string().await.unwrap());
    assert!(blinded_signature_response.is_ok());
}

#[tokio::test]
async fn verification_of_bandwidth_credential() {
    // Setup variables
    let validator_address = AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap();
    let proposal_db = Arc::new(RwLock::new(HashMap::new()));
    let spent_credential_db = Arc::new(RwLock::new(HashMap::new()));
    let nyxd_client = DummyClient::new(validator_address.clone())
        .with_proposal_db(&proposal_db)
        .with_spent_credential_db(&spent_credential_db);
    let mut db_dir = std::env::temp_dir();
    let params = Parameters::new(4).unwrap();
    let mut key_pairs = ttp_keygen(&params, 1, 1).unwrap();
    let voucher_value = 1234u64;
    let voucher_info = "voucher info";
    let public_attributes = vec![
        hash_to_scalar(voucher_value.to_string()),
        hash_to_scalar(voucher_info),
    ];
    let indices: Vec<u64> = key_pairs
        .iter()
        .enumerate()
        .map(|(idx, _)| (idx + 1) as u64)
        .collect();
    let theta =
        theta_from_keys_and_attributes(&params, &key_pairs, &indices, &public_attributes).unwrap();
    let key_pair = key_pairs.remove(0);
    db_dir.push(&key_pair.verification_key().to_bs58()[..8]);
    let storage1 = NymApiStorage::init(db_dir).await.unwrap();
    let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key());
    let staged_key_pair = crate::coconut::KeyPair::new();
    staged_key_pair.set(Some(key_pair)).await;
    let rocket = rocket::build().attach(InternalSignRequest::stage(
        nyxd_client.clone(),
        TEST_COIN_DENOM.to_string(),
        staged_key_pair,
        comm_channel.clone(),
        storage1.clone(),
    ));

    let client = Client::tracked(rocket)
        .await
        .expect("valid rocket instance");

    let credential = Credential::new(4, theta.clone(), voucher_value, voucher_info.to_string(), 0);
    let proposal_id = 42;
    // The address is not used, so we can use a duplicate
    let gateway_cosmos_addr = validator_address.clone();
    let req =
        VerifyCredentialBody::new(credential.clone(), proposal_id, gateway_cosmos_addr.clone());

    // Test endpoint with not proposal for the proposal id
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().await.unwrap(),
        CoconutError::IncorrectProposal {
            reason: "proposal not found".to_string()
        }
        .to_string()
    );

    let mut proposal = ProposalResponse {
        id: proposal_id,
        title: String::new(),
        description: String::from("25mnnoCcUfeizfC85avvroFg2prpEZBgJbJM2SLtkgyyUkoAU3cqJiqWmg8cMHEPjfFf5sQF92SMAM2vbEoLZvUjenvXhadTLdA4TqMYArJpihyqirW2AhGoNehtcdcK5gnH"),
        msgs: vec![],
        status: cw3::Status::Open,
        expires: cw_utils::Expiration::Never {},
        threshold: cw_utils::ThresholdResponse::AbsolutePercentage {
            percentage: Decimal::from_ratio(2u32, 3u32),
            total_weight: 100,
        },
        proposer: Addr::unchecked("proposer"),
        deposit: None,
    };

    // Test the endpoint with a different blinded serial number in the description
    proposal_db
        .write()
        .unwrap()
        .insert(proposal_id, proposal.clone());
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().await.unwrap(),
        CoconutError::IncorrectProposal {
            reason: "incorrect blinded serial number in description".to_string()
        }
        .to_string()
    );

    // Test the endpoint with no msg in the proposal action
    proposal.description = credential.blinded_serial_number();
    proposal_db
        .write()
        .unwrap()
        .insert(proposal_id, proposal.clone());
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().await.unwrap(),
        CoconutError::IncorrectProposal {
            reason: "action is not to release funds".to_string()
        }
        .to_string()
    );

    // Test the endpoint without any credential recorded in the Coconut Bandwidth Contract
    let funds = Coin::new(voucher_value as u128, TEST_COIN_DENOM);
    let msg = nym_coconut_bandwidth_contract_common::msg::ExecuteMsg::ReleaseFunds {
        funds: funds.clone().into(),
    };
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: String::new(),
        msg: to_binary(&msg).unwrap(),
        funds: vec![],
    });
    proposal.msgs = vec![cosmos_msg];
    proposal_db
        .write()
        .unwrap()
        .insert(proposal_id, proposal.clone());
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().await.unwrap(),
        CoconutError::InvalidCredentialStatus {
            status: "spent credential not found".to_string()
        }
        .to_string()
    );

    spent_credential_db.write().unwrap().insert(
        credential.blinded_serial_number(),
        SpendCredentialResponse::new(None),
    );
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().await.unwrap(),
        CoconutError::InvalidCredentialStatus {
            status: "Inexistent".to_string()
        }
        .to_string()
    );

    // Test the endpoint with a credential that doesn't verify correctly
    let mut spent_credential = SpendCredential::new(
        funds.clone().into(),
        credential.blinded_serial_number(),
        Addr::unchecked("unimportant"),
    );
    spent_credential_db.write().unwrap().insert(
        credential.blinded_serial_number(),
        SpendCredentialResponse::new(Some(spent_credential.clone())),
    );
    let bad_credential = Credential::new(
        4,
        theta.clone(),
        voucher_value,
        String::from("bad voucher info"),
        0,
    );
    let bad_req =
        VerifyCredentialBody::new(bad_credential, proposal_id, gateway_cosmos_addr.clone());
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&bad_req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let verify_credential_response =
        serde_json::from_str::<VerifyCredentialResponse>(&response.into_string().await.unwrap())
            .unwrap();
    assert!(!verify_credential_response.verification_result);
    assert_eq!(
        cw3::Status::Rejected,
        proposal_db
            .read()
            .unwrap()
            .get(&proposal_id)
            .unwrap()
            .status
    );

    // Test the endpoint with a proposal that has a different value for the funds to be released
    // then what's in the credential
    let funds = Coin::new((voucher_value + 10) as u128, TEST_COIN_DENOM);
    let msg = nym_coconut_bandwidth_contract_common::msg::ExecuteMsg::ReleaseFunds {
        funds: funds.clone().into(),
    };
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: String::new(),
        msg: to_binary(&msg).unwrap(),
        funds: vec![],
    });
    proposal.msgs = vec![cosmos_msg];
    proposal_db
        .write()
        .unwrap()
        .insert(proposal_id, proposal.clone());
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let verify_credential_response =
        serde_json::from_str::<VerifyCredentialResponse>(&response.into_string().await.unwrap())
            .unwrap();
    assert!(!verify_credential_response.verification_result);
    assert_eq!(
        cw3::Status::Rejected,
        proposal_db
            .read()
            .unwrap()
            .get(&proposal_id)
            .unwrap()
            .status
    );

    // Test the endpoint with every dependency met
    let funds = Coin::new(voucher_value as u128, TEST_COIN_DENOM);
    let msg = nym_coconut_bandwidth_contract_common::msg::ExecuteMsg::ReleaseFunds {
        funds: funds.clone().into(),
    };
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: String::new(),
        msg: to_binary(&msg).unwrap(),
        funds: vec![],
    });
    proposal.msgs = vec![cosmos_msg];
    proposal_db
        .write()
        .unwrap()
        .insert(proposal_id, proposal.clone());
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let verify_credential_response =
        serde_json::from_str::<VerifyCredentialResponse>(&response.into_string().await.unwrap())
            .unwrap();
    assert!(verify_credential_response.verification_result);
    assert_eq!(
        cw3::Status::Passed,
        proposal_db
            .read()
            .unwrap()
            .get(&proposal_id)
            .unwrap()
            .status
    );

    // Test the endpoint with the credential marked as Spent in the Coconut Bandwidth Contract
    spent_credential.mark_as_spent();
    spent_credential_db.write().unwrap().insert(
        credential.blinded_serial_number(),
        SpendCredentialResponse::new(Some(spent_credential)),
    );
    let response = client
        .post(format!(
            "/{}/{}/{}/{}",
            API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
        ))
        .json(&req)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().await.unwrap(),
        CoconutError::InvalidCredentialStatus {
            status: "Spent".to_string()
        }
        .to_string()
    );
}
