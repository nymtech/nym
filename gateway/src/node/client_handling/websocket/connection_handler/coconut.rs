// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::Storage;

use super::authenticated::RequestHandlingError;
use chrono::Utc;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use log::*;
use nym_api_requests::coconut::OfflineVerifyCredentialBody;
use nym_credentials::coconut::bandwidth::bandwidth_credential_params;
use nym_credentials::coconut::utils::today_timestamp;
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::{CompactEcashError, PayInfo, VerificationKeyAuth};
use nym_gateway_requests::models::{CredentialSpendingRequest, OldCredentialSpendingRequest};
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::{MultisigQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::{AccountId, Coin};
use nym_validator_client::{
    nyxd::{
        contract_traits::{CoconutBandwidthSigningClient, DkgQueryClient, MultisigSigningClient},
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient, NymApiClient,
};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::time::{interval, Duration};

const TIME_RANGE_SEC: i64 = 30;
const CRED_SENDING_INTERVAL: u64 = 300;

pub(crate) struct EcashVerifier {
    address: AccountId,
    nyxd_client: RwLock<DirectSigningHttpRpcNyxdClient>,

    // **CURRENTLY** api client addresses don't change during the epochs
    api_clients: RwLock<HashMap<EpochId, Vec<CoconutApiClient>>>,

    // keys never change during epochs
    master_keys: RwLock<HashMap<EpochId, VerificationKeyAuth>>,
    mix_denom_base: String,
    pk_bytes: [u8; 32], //bytes represenation of a pub key representing the verifier
    pay_infos: Arc<Mutex<Vec<PayInfo>>>,
    cred_sender: UnboundedSender<PendingCredential>,
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
        let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();
        let address = nyxd_client.address();
        let (cred_sender, cred_receiver) = mpsc::unbounded();
        //initialize a credential sender only if we are in offline mode
        if offline_verification {
            let cs = CredentialSender::new(cred_receiver, storage);
            cs.start(shutdown);
        } else {
            shutdown.mark_as_success();
        }

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
                nyxd_client: RwLock::new(nyxd_client),
                api_clients: Default::default(),
                master_keys: Default::default(),
                mix_denom_base,
                pk_bytes,
                pay_infos: Arc::new(Mutex::new(Vec::new())),
                cred_sender,
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
                nyxd_client: RwLock::new(nyxd_client),
                api_clients: Default::default(),
                master_keys: Default::default(),
                mix_denom_base,
                pk_bytes,
                pay_infos: Arc::new(Mutex::new(Vec::new())),
                cred_sender,
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

            api_clients.insert(current_epoch.epoch_id, epoch_api_clients);
            master_keys.insert(current_epoch.epoch_id, aggregated_verification_key);
        }

        Ok(EcashVerifier {
            address,
            nyxd_client: RwLock::new(nyxd_client),
            api_clients: RwLock::new(api_clients),
            master_keys: RwLock::new(master_keys),
            mix_denom_base,
            pk_bytes,
            pay_infos: Arc::new(Mutex::new(Vec::new())),
            cred_sender,
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
        guard.insert(epoch_id, api_clients);
        let guard = guard.downgrade();
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
        let insert_index = self.verify_pay_info(credential.pay_info.clone()).await?;

        credential
            .verify(
                bandwidth_credential_params(),
                aggregated_verification_key,
                today_timestamp(),
            )
            .map_err(|err| match err {
                CompactEcashError::ExpirationDate(_) => {
                    RequestHandlingError::InvalidBandwidthCredential(String::from(
                        "credential failed to verify on gateway - past expiration date",
                    ))
                }
                _ => RequestHandlingError::InvalidBandwidthCredential(String::from(
                    "credential failed to verify on gateway",
                )),
            })?;

        self.insert_pay_info(credential.pay_info.clone(), insert_index)
            .await
    }

    pub async fn verify_pay_info(&self, pay_info: PayInfo) -> Result<usize, RequestHandlingError> {
        //Public key check
        if pay_info.pk() != self.pk_bytes {
            return Err(RequestHandlingError::InvalidBandwidthCredential(
                String::from("credential failed to verify on gateway - invalid provider pk"),
            ));
        }

        //Timestamp range check
        let timestamp = Utc::now().timestamp();
        let tmin = timestamp - TIME_RANGE_SEC;
        let tmax = timestamp + TIME_RANGE_SEC;
        if pay_info.timestamp() > tmax || pay_info.timestamp() < tmin {
            return Err(RequestHandlingError::InvalidBandwidthCredential(
                String::from("credential failed to verify on gateway - invalid timestamp"),
            ));
        }

        let mut inner = self
            .pay_infos
            .lock()
            .map_err(|_| RequestHandlingError::InternalError)?;

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
                    return Err(RequestHandlingError::InvalidBandwidthCredential(
                        String::from("credential failed to verify on gateway - duplicate payInfo"),
                    ));
                }
                //tbh, I don't expect ending up here if all parties are honest
                //binary search returns an arbitrary match, so we have to check for potential multiple matches
                let mut i = index as i64;
                while i >= 0 && inner[i as usize].timestamp() == pay_info.timestamp() {
                    if inner[i as usize] == pay_info {
                        return Err(RequestHandlingError::InvalidBandwidthCredential(
                            String::from(
                                "credential failed to verify on gateway - duplicate payInfo",
                            ),
                        ));
                    }
                    i -= 1;
                }

                let mut i = index + 1;
                while i < inner.len() && inner[i].timestamp() == pay_info.timestamp() {
                    if inner[i] == pay_info {
                        return Err(RequestHandlingError::InvalidBandwidthCredential(
                            String::from(
                                "credential failed to verify on gateway - duplicate payInfo",
                            ),
                        ));
                    }
                    i += 1;
                }
                Ok(index)
            }
        }
    }

    pub async fn insert_pay_info(
        &self,
        pay_info: PayInfo,
        index: usize,
    ) -> Result<(), RequestHandlingError> {
        let mut inner = self
            .pay_infos
            .lock()
            .map_err(|_| RequestHandlingError::InternalError)?;
        if index > inner.len() {
            inner.push(pay_info);
            return Ok(());
        }
        inner.insert(index, pay_info);
        Ok(())
    }

    pub async fn post_credential(
        &self,
        api_clients: &[CoconutApiClient],
        credential: CredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        for client in api_clients {
            self.cred_sender
                .unbounded_send(PendingCredential {
                    credential: credential.clone(),
                    address: self.nyxd_client.read().await.address(),
                    client: client.api_client.clone(),
                })
                .map_err(|_| RequestHandlingError::InternalError)?
        }
        Ok(())
    }

    pub async fn release_bandwidth_voucher_funds(
        &self,
        api_clients: &[CoconutApiClient],
        credential: &CredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        if !credential.data.typ.is_ticketbook() {
            unimplemented!()
        }

        let voucher_amount = credential.data.value as u128;

        let serial_number = credential.data.payment.serial_number_bs58();

        let res = self
            .nyxd_client
            .write()
            .await
            .spend_credential(
                Coin::new(voucher_amount, &self.mix_denom_base),
                serial_number,
                self.address.to_string(),
                None,
            )
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

        let req = nym_api_requests::coconut::OnlineVerifyCredentialBody::new(
            credential.data.clone(),
            proposal_id,
            self.address.clone(),
        );
        for client in api_clients {
            let ret = client.api_client.verify_online_credential(&req).await;
            let client_url = client.api_client.nym_api.current_url();
            match ret {
                Ok(res) => {
                    if !res.verification_result {
                        warn!("Validator at {client_url} didn't accept the credential. It will probably vote No on the spending proposal");
                    }
                }
                Err(err) => {
                    warn!("Validator at {client_url} could not be reached. There might be a problem with the coconut endpoint: {err}");
                }
            }
        }

        self.nyxd_client
            .write()
            .await
            .execute_proposal(proposal_id, None)
            .await?;

        Ok(())
    }

    pub async fn release_old_bandwidth_voucher_funds(
        &self,
        api_clients: &[CoconutApiClient],
        credential: OldCredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        if !credential.data.typ.is_ticketbook() {
            unimplemented!()
        }

        // safety: the voucher funds are released after the credential has already been verified locally
        // and the underlying bandwidth value has been extracted, so the below MUST succeed
        let voucher_amount = credential.unchecked_voucher_value() as u128;

        let blinded_serial_number = credential
            .data
            .verify_credential_request
            .blinded_serial_number_bs58();

        let res = self
            .nyxd_client
            .write()
            .await
            .spend_credential(
                Coin::new(voucher_amount, &self.mix_denom_base),
                blinded_serial_number,
                self.address.to_string(),
                None,
            )
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
        if !credential.matches_blinded_serial_number(&proposal.description)? {
            return Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal has different serial number"),
            });
        }

        let req = nym_api_requests::coconut::models::VerifyCredentialBody::new(
            credential.data,
            proposal_id,
            self.address.clone(),
        );
        for client in api_clients {
            let ret = client.api_client.verify_bandwidth_credential(&req).await;
            let client_url = client.api_client.nym_api.current_url();
            match ret {
                Ok(res) => {
                    if !res.verification_result {
                        warn!("Validator at {client_url} didn't accept the credential. It will probably vote No on the spending proposal");
                    }
                }
                Err(err) => {
                    warn!("Validator at {client_url} could not be reached. There might be a problem with the coconut endpoint: {err}");
                }
            }
        }

        self.nyxd_client
            .write()
            .await
            .execute_proposal(proposal_id, None)
            .await?;

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct PendingCredential {
    pub(crate) credential: CredentialSpendingRequest,
    pub(crate) address: AccountId,
    pub(crate) client: NymApiClient,
}

struct CredentialSender<St: Storage> {
    cred_receiver: UnboundedReceiver<PendingCredential>,
    storage: St,
}

impl<St> CredentialSender<St>
where
    St: Storage + 'static,
{
    fn new(cred_receiver: UnboundedReceiver<PendingCredential>, storage: St) -> Self {
        CredentialSender {
            cred_receiver,
            storage,
        }
    }

    async fn send_credential(pending: &PendingCredential) -> bool {
        let request = OfflineVerifyCredentialBody::new(
            pending.credential.data.clone(),
            pending.address.clone(),
        );
        match pending.client.verify_offline_credential(&request).await {
            Ok(res) => {
                if !res.verification_result {
                    log::debug!(
                        "Validator {} didn't accept the credential.",
                        pending.client.nym_api.current_url()
                    );
                }
                //Credential was sent
                true
            }
            Err(e) => {
                log::warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", pending.client.nym_api.current_url(), e);
                false
            }
        }
    }
    async fn handle_credential(&mut self, pending: PendingCredential) {
        if !Self::send_credential(&pending).await {
            //failed to send, store credential
            if let Err(err) = self.storage.insert_pending_credential(pending).await {
                log::error!("Failed to store pending credential - {:?}", err);
            };
        }
    }

    async fn try_empty_pending(&mut self) {
        log::debug!("Trying to send unsent payments");
        let pending = match self.storage.get_all_pending_credential().await {
            Err(err) => {
                log::error!("Failed to retrieve pending credential - {:?}", err);
                return;
            }
            Ok(res) => res,
        };

        for (id, pending) in pending {
            if Self::send_credential(&pending).await {
                //send successful, remove credential from storage
                if let Err(err) = self.storage.remove_pending_credential(id).await {
                    log::error!("Failed to remove pending credential - {:?}", err);
                }
            }
        }
    }

    async fn run(mut self, mut shutdown: nym_task::TaskClient) {
        log::info!("Starting Ecash CredentialSender");
        let mut interval = interval(Duration::from_secs(CRED_SENDING_INTERVAL));

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("client_handling::credentialSender : received shutdown");
                },
                Some(credential) = self.cred_receiver.next() => self.handle_credential(credential).await,
                _ = interval.tick() => self.try_empty_pending().await,

            }
        }
    }

    fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
