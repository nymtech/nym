// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::Storage;

use futures::channel::mpsc::{self, UnboundedSender};
use log::*;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

use super::authenticated::RequestHandlingError;
use nym_api_requests::coconut::models::VerifyEcashCredentialResponse;
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::{
    CompactEcashError, CredentialType, NymPayInfo, VerificationKeyAuth,
};
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::{
    EcashSigningClient, MultisigQueryClient, NymContractsProvider,
};
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{
    nyxd::{
        contract_traits::{DkgQueryClient, MultisigSigningClient},
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient,
};

use credential_sender::CredentialSender;
use double_spending::DoubleSpendingDetector;

pub use credential_sender::PendingCredential;

mod credential_sender;
mod double_spending;

const TIME_RANGE_SEC: i64 = 30;

pub struct EcashVerifier {
    address: AccountId,
    nyxd_client: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,

    // **CURRENTLY** api client addresses don't change during the epochs
    api_clients: RwLock<HashMap<EpochId, Vec<CoconutApiClient>>>,

    // keys never change during epochs
    master_keys: RwLock<HashMap<EpochId, VerificationKeyAuth>>,
    pk_bytes: [u8; 32], //bytes represenation of a pub key representing the verifier
    pay_infos: Mutex<Vec<NymPayInfo>>,
    cred_sender: UnboundedSender<PendingCredential>,
    double_spend_detector: Option<DoubleSpendingDetector>,
}

impl EcashVerifier {
    pub async fn new<St: Storage + 'static>(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        only_coconut_credentials: bool,
        pk_bytes: [u8; 32],
        mut shutdown: nym_task::TaskClient,
        storage: St,
        offline_verification: bool,
    ) -> Result<Self, RequestHandlingError> {
        let address = nyxd_client.address();

        let double_spend_detector = if offline_verification {
            let inner = DoubleSpendingDetector::new();
            inner.clone().start(shutdown.clone());
            Some(inner)
        } else {
            None
        };

        let (cred_sender, cred_receiver) = mpsc::unbounded();

        let mut master_keys = HashMap::new();
        let mut api_clients = HashMap::new();

        // don't make it a hard failure in case we're running on mainnet (where DKG hasn't been deployed yet)
        if nyxd_client.dkg_contract_address().is_none() {
            if !only_coconut_credentials {
                warn!(
                    "the DKG contract address is not available - \
                no coconut credentials will be redeemable \
                (if the DKG ceremony hasn't been run yet this warning is expected)"
                );
            } else {
                // if we require coconut credentials, we MUST have DKG contract available
                return Err(RequestHandlingError::UnavailableDkgContract);
            }

            return Ok(EcashVerifier {
                address,
                nyxd_client: Arc::new(RwLock::new(nyxd_client)),
                api_clients: Default::default(),
                master_keys: Default::default(),
                pk_bytes,
                pay_infos: Default::default(),
                cred_sender,
                double_spend_detector,
            });
        }

        let Ok(current_epoch) = nyxd_client.get_current_epoch().await else {
            // another case of somebody putting a placeholder address that doesn't exist
            error!("the specified DKG contract address is invalid - no coconut credentials will be redeemable");
            if only_coconut_credentials {
                // if we require coconut credentials, we MUST have DKG contract available
                return Err(RequestHandlingError::UnavailableDkgContract);
            }
            return Ok(EcashVerifier {
                address,
                nyxd_client: Arc::new(RwLock::new(nyxd_client)),
                api_clients: Default::default(),
                master_keys: Default::default(),
                pk_bytes,
                pay_infos: Default::default(),
                cred_sender,
                double_spend_detector,
            });
        };

        // might as well obtain the key for the current epoch, if applicable
        if current_epoch.state.is_in_progress() {
            // note: even though we're constructing clients here, we will NOT be making any network requests
            let epoch_api_clients =
                all_ecash_api_clients(&nyxd_client, current_epoch.epoch_id).await?;
            let threshold = nyxd_client.get_current_epoch_threshold().await?;

            // SAFETY:
            // if epoch state is in the 'in progress' state, it means the threshold value MUST HAVE
            // been established. if it wasn't, there's an underlying issue with the DKG contract in which
            // case we shouldn't continue anyway because here be dragons
            #[allow(clippy::expect_used)]
            let threshold = threshold.expect("unavailable threshold value") as usize;
            if epoch_api_clients.len() < threshold {
                return Err(RequestHandlingError::NotEnoughNymAPIs {
                    received: epoch_api_clients.len(),
                    needed: threshold,
                });
            }
            let aggregated_verification_key =
                nym_credentials::obtain_aggregate_verification_key(&epoch_api_clients)?;

            api_clients.insert(current_epoch.epoch_id, epoch_api_clients.clone());
            if let Some(detector) = &double_spend_detector {
                detector
                    .update_api_client(current_epoch.epoch_id, epoch_api_clients)
                    .await;
            }
            master_keys.insert(current_epoch.epoch_id, aggregated_verification_key);
        }

        let nyxd_client = Arc::new(RwLock::new(nyxd_client));
        //initialize a credential sender only if we are in offline mode
        if offline_verification {
            let cs = CredentialSender::new(cred_receiver, storage, nyxd_client.clone());
            cs.start(shutdown);
        } else {
            shutdown.mark_as_success();
        }

        Ok(EcashVerifier {
            address,
            nyxd_client,
            api_clients: RwLock::new(api_clients),
            master_keys: RwLock::new(master_keys),
            pk_bytes,
            pay_infos: Default::default(),
            cred_sender,
            double_spend_detector,
        })
    }

    pub async fn api_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<Vec<CoconutApiClient>>, RequestHandlingError> {
        let guard = self.api_clients.read().await;

        // the key was already in the map
        if let Ok(mapped) = RwLockReadGuard::try_map(guard, |clients| clients.get(&epoch_id)) {
            trace!("we already had cached api clients for epoch {epoch_id}");
            return Ok(mapped);
        }

        let api_clients = self.query_api_clients(epoch_id).await?;
        trace!(
            "obtained {} api clients for epoch {epoch_id} from the contract",
            api_clients.len()
        );

        // EDGE CASE:
        // if this epoch is from the past, we can't query for its threshold
        // we can only hope that enough valid keys were submitted
        // the best we can do is check if we have at least a api
        if api_clients.is_empty() {
            return Err(RequestHandlingError::NotEnoughNymAPIs {
                received: 0,
                needed: 1,
            });
        }

        let mut guard = self.api_clients.write().await;
        guard.insert(epoch_id, api_clients.clone());
        let guard = guard.downgrade();
        if let Some(detector) = &self.double_spend_detector {
            detector.update_api_client(epoch_id, api_clients).await;
        }
        trace!("stored api clients for epoch {epoch_id}");

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |clients| {
            clients.get(&epoch_id).unwrap()
        }))
    }

    pub async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>, RequestHandlingError> {
        let guard = self.master_keys.read().await;

        // the key was already in the map
        if let Ok(mapped) = RwLockReadGuard::try_map(guard, |keys| keys.get(&epoch_id)) {
            trace!("we already had cached verification key for epoch {epoch_id}");
            return Ok(mapped);
        }

        let api_clients = self.api_clients(epoch_id).await?;
        trace!(
            "attempting to obtain verification key from {} api clients",
            api_clients.len()
        );

        let aggregated_verification_key =
            nym_credentials::obtain_aggregate_verification_key(&api_clients)?;

        let mut guard = self.master_keys.write().await;
        guard.insert(epoch_id, aggregated_verification_key);
        let guard = guard.downgrade();
        trace!("stored aggregated verification key for epoch {epoch_id}");

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |keys| {
            keys.get(&epoch_id).unwrap()
        }))
    }

    pub async fn query_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_ecash_api_clients(self.nyxd_client.read().await.deref(), epoch_id).await?)
    }

    //Check for duplicate pay_info, then check the payment, then insert pay_info if everything succeeded
    pub async fn check_payment(
        &self,
        credential: &CredentialSpendingData,
        aggregated_verification_key: &VerificationKeyAuth,
    ) -> Result<(), RequestHandlingError> {
        let insert_index = self.verify_pay_info(credential.pay_info.into()).await?;

        credential
            .verify(aggregated_verification_key)
            .map_err(|err| match err {
                CompactEcashError::ExpirationDateSignatureValidity => {
                    RequestHandlingError::InvalidBandwidthCredential(String::from(
                        "credential failed to verify on gateway - past expiration date",
                    ))
                }
                _ => RequestHandlingError::InvalidBandwidthCredential(String::from(
                    "credential failed to verify on gateway",
                )),
            })?;

        self.insert_pay_info(credential.pay_info.into(), insert_index)
            .await
    }

    pub async fn verify_pay_info(
        &self,
        pay_info: NymPayInfo,
    ) -> Result<usize, RequestHandlingError> {
        //Public key check
        if pay_info.pk() != self.pk_bytes {
            return Err(RequestHandlingError::InvalidPayInfoPublicKey);
        }

        //Timestamp range check
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let tmin = timestamp - TIME_RANGE_SEC;
        let tmax = timestamp + TIME_RANGE_SEC;
        if pay_info.timestamp() > tmax || pay_info.timestamp() < tmin {
            return Err(RequestHandlingError::InvalidPayInfoTimestamp);
        }

        let mut inner = self.pay_infos.lock().await;

        //Cleanup inner
        let low = inner.partition_point(|x| x.timestamp() < tmin);
        let high = inner.partition_point(|x| x.timestamp() < tmax);
        inner.truncate(high);
        drop(inner.drain(..low));

        //Duplicate check
        match inner.binary_search_by(|info| info.timestamp().cmp(&pay_info.timestamp())) {
            Result::Err(index) => Ok(index),
            Result::Ok(index) => {
                if inner[index] == pay_info {
                    return Err(RequestHandlingError::DuplicatePayInfo);
                }
                //tbh, I don't expect ending up here if all parties are honest
                //binary search returns an arbitrary match, so we have to check for potential multiple matches
                let mut i = index as i64;
                while i >= 0 && inner[i as usize].timestamp() == pay_info.timestamp() {
                    if inner[i as usize] == pay_info {
                        return Err(RequestHandlingError::DuplicatePayInfo);
                    }
                    i -= 1;
                }

                let mut i = index + 1;
                while i < inner.len() && inner[i].timestamp() == pay_info.timestamp() {
                    if inner[i] == pay_info {
                        return Err(RequestHandlingError::DuplicatePayInfo);
                    }
                    i += 1;
                }
                Ok(index)
            }
        }
    }

    async fn insert_pay_info(
        &self,
        pay_info: NymPayInfo,
        index: usize,
    ) -> Result<(), RequestHandlingError> {
        let mut inner = self.pay_infos.lock().await;
        if index > inner.len() {
            inner.push(pay_info);
            return Ok(());
        }
        inner.insert(index, pay_info);
        Ok(())
    }

    pub async fn check_double_spend(&self, serial_number_bs58: &String) -> bool {
        if let Some(double_spend_detector) = &self.double_spend_detector {
            double_spend_detector.check(serial_number_bs58).await
        } else {
            false
        }
    }

    pub async fn post_credential(
        &self,
        api_clients: &[CoconutApiClient],
        credential: CredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        self.cred_sender
            .unbounded_send(PendingCredential::new(
                credential.clone(),
                self.nyxd_client.read().await.address(),
                api_clients
                    .iter()
                    .map(|client| client.api_client.clone())
                    .collect(),
            ))
            .map_err(|_| RequestHandlingError::InternalError)
    }

    pub async fn spend_online_credential(
        &self,
        api_clients: &[CoconutApiClient],
        credential: &CredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        let serial_number = credential.data.payment.serial_number_bs58();
        let proposal_id = match credential.data.typ {
            CredentialType::TicketBook => {
                let res = self
                    .nyxd_client
                    .write()
                    .await
                    .prepare_credential(serial_number, self.address.to_string(), None)
                    .await?;
                let proposal_id = find_attribute(&res.logs, "wasm", BANDWIDTH_PROPOSAL_ID)
                    .ok_or(RequestHandlingError::ProposalIdError {
                        reason: String::from("proposal id not found"),
                    })?
                    .value
                    .parse::<u64>()
                    .map_err(|_| RequestHandlingError::ProposalIdError {
                        reason: String::from("proposal id could not be parsed to u64"),
                    })?;

                let proposal = self
                    .nyxd_client
                    .read()
                    .await
                    .query_proposal(proposal_id)
                    .await?;
                if !credential.matches_serial_number(&proposal.description)? {
                    return Err(RequestHandlingError::ProposalIdError {
                        reason: String::from("proposal has different serial number"),
                    });
                }
                Some(proposal_id)
            }
            CredentialType::FreePass => None,
        };

        let req = nym_api_requests::coconut::VerifyEcashCredentialBody::new(
            credential.data.clone(),
            self.address.clone(),
            proposal_id,
        );
        for client in api_clients {
            let ret = client.api_client.verify_online_credential(&req).await;
            let client_url = client.api_client.nym_api.current_url();
            match ret {
                Ok(VerifyEcashCredentialResponse::Accepted) => {
                    debug!("Validator at {client_url} accepted the credential");
                }
                Ok(response) => {
                    debug!("Validator at {client_url} didn't accept the credential. Reason : {response}");
                }
                Err(err) => {
                    warn!("Validator at {client_url} could not be reached. There might be a problem with the coconut endpoint: {err}");
                }
            }
        }

        if let Some(proposal_id) = proposal_id {
            if self
                .nyxd_client
                .read()
                .await
                .query_proposal(proposal_id)
                .await?
                .status
                == nym_validator_client::nyxd::cw3::Status::Rejected
            {
                return Err(RequestHandlingError::InvalidBandwidthCredential(
                    "Refused by validators".to_string(),
                ));
            }
            self.nyxd_client
                .write()
                .await
                .execute_proposal(proposal_id, None)
                .await?;
        }

        Ok(())
    }
}
