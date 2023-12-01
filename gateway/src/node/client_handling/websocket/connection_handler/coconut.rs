// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::Storage;

use super::authenticated::RequestHandlingError;
use chrono::Utc;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use log::*;
use nym_api_requests::coconut::OfflineVerifyCredentialBody;
use nym_compact_ecash::scheme::EcashCredential;
use nym_compact_ecash::setup::Parameters;
use nym_compact_ecash::{PayInfo, VerificationKeyAuth};
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nyxd::{AccountId, Coin, Fee};
use nym_validator_client::{
    nyxd::{
        contract_traits::{
            CoconutBandwidthSigningClient, DkgQueryClient, MultisigQueryClient,
            MultisigSigningClient,
        },
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient, NymApiClient,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use tokio::time::{interval, Duration};

const TIME_RANGE_SEC: i64 = 30;
const CRED_SENDING_INTERVAL: u64 = 300;
const ONE_HOUR_SEC: u64 = 3600;
const MAX_FEEGRANT_UNYM: u128 = 10000;

pub(crate) struct EcashVerifier {
    nyxd_client: DirectSigningHttpRpcNyxdClient,
    mix_denom_base: String,
    ecash_parameters: Parameters,
    pk_bytes: [u8; 32], //bytes represenation of a pub key representing the verifier
    pay_infos: Arc<Mutex<Vec<PayInfo>>>,
    cred_sender: UnboundedSender<PendingCredential>,
}

impl EcashVerifier {
    pub fn new<St: Storage + 'static>(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        ecash_parameters: Parameters,
        pk_bytes: [u8; 32],
        mut shutdown: nym_task::TaskClient,
        storage: St,
        offline_verification: bool,
    ) -> Self {
        let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();
        let (cred_sender, cred_receiver) = mpsc::unbounded();
        //SW do not initialize unused elements
        if offline_verification {
            let cs = CredentialSender::new(cred_receiver, storage);
            cs.start(shutdown);
        } else {
            shutdown.mark_as_success();
        }

        EcashVerifier {
            nyxd_client,
            mix_denom_base,
            ecash_parameters,
            pk_bytes,
            pay_infos: Arc::new(Mutex::new(Vec::new())),
            cred_sender,
        }
    }

    pub async fn all_current_ecash_api_clients(
        &self,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        let epoch_id = self.nyxd_client.get_current_epoch().await?.epoch_id;
        self.all_ecash_api_clients(epoch_id).await
    }

    pub async fn all_ecash_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_ecash_api_clients(&self.nyxd_client, epoch_id).await?)
    }

    //Check for duplicate pay_info, then check the payment, then insert pay_info if everything succeeded
    pub async fn check_payment(
        &self,
        credential: &EcashCredential,
        aggregated_verification_key: &VerificationKeyAuth,
    ) -> Result<(), RequestHandlingError> {
        let insert_index = self.verify_pay_info(credential.pay_info().clone()).await?;

        credential
            .payment()
            .spend_verify(
                &self.ecash_parameters,
                aggregated_verification_key,
                credential.pay_info(),
            )
            .map_err(|_| {
                RequestHandlingError::InvalidBandwidthCredential(String::from(
                    "credential failed to verify on gateway",
                ))
            })?;

        self.insert_pay_info(credential.pay_info().clone(), insert_index)
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

    pub fn post_credential(
        &self,
        api_clients: Vec<CoconutApiClient>,
        credential: EcashCredential,
    ) -> Result<(), RequestHandlingError> {
        for client in api_clients {
            self.cred_sender
                .unbounded_send(PendingCredential {
                    credential: credential.clone(),
                    address: self.nyxd_client.address(),
                    client: client.api_client,
                })
                .map_err(|_| RequestHandlingError::InternalError)?
        }
        Ok(())
    }

    pub async fn release_funds(
        &self,
        api_clients: Vec<CoconutApiClient>,
        credential: &EcashCredential,
    ) -> Result<(), RequestHandlingError> {
        // Use a custom multiplier for revoke, as the default one (1.3)
        // isn't enough
        let revoke_fee = Some(Fee::Auto(Some(1.5)));

        let res = self
            .nyxd_client
            .spend_credential(
                Coin::new(credential.value().into(), self.mix_denom_base.clone()),
                credential.serial_number(),
                self.nyxd_client.address().to_string(),
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

        let proposal = self.nyxd_client.query_proposal(proposal_id).await?;
        if !credential.has_serial_number(&proposal.description)? {
            return Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal has different serial number"),
            });
        }

        let req = nym_api_requests::coconut::OnlineVerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nyxd_client.address(),
        );
        for client in api_clients {
            self.nyxd_client
                .grant_allowance(
                    &client.cosmos_address,
                    vec![Coin::new(MAX_FEEGRANT_UNYM, self.mix_denom_base.clone())],
                    SystemTime::now().checked_add(Duration::from_secs(ONE_HOUR_SEC)),
                    // It would be nice to be able to filter deeper, but for now only the msg type filter is avaialable
                    vec![String::from("/cosmwasm.wasm.v1.MsgExecuteContract")],
                    "Create allowance to vote the release of funds".to_string(),
                    None,
                )
                .await?;
            let ret = client.api_client.verify_online_credential(&req).await;
            self.nyxd_client
                .revoke_allowance(
                    &client.cosmos_address,
                    "Cleanup the previous allowance for releasing funds".to_string(),
                    revoke_fee.clone(),
                )
                .await?;
            match ret {
                Ok(res) => {
                    if !res.verification_result {
                        debug!("Validator {} didn't accept the credential. It will probably vote No on the spending proposal", client.api_client.nym_api.current_url());
                    }
                }
                Err(e) => {
                    warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", client.api_client.nym_api.current_url(), e);
                }
            }
        }

        self.nyxd_client.execute_proposal(proposal_id, None).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct PendingCredential {
    pub(crate) credential: EcashCredential,
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
        let request =
            OfflineVerifyCredentialBody::new(pending.credential.clone(), pending.address.clone());
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
