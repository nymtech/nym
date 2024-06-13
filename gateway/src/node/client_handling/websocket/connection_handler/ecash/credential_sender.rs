// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::authenticated::RequestHandlingError;
use crate::node::client_handling::websocket::connection_handler::ecash::state::SharedState;
use crate::node::storage::Storage;
use futures::channel::mpsc::UnboundedReceiver;
use futures::StreamExt;
use log::{debug, error, warn};
use nym_api_requests::coconut::models::{
    BatchRedeemTicketsBody, EcachTicketVerificationRejection, VerifyEcashTicketBody,
};
use nym_api_requests::coconut::VerifyEcashCredentialBody;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_validator_client::nyxd::contract_traits::{EcashSigningClient, MultisigQueryClient};
use nym_validator_client::nyxd::cosmwasm_client::logs::find_proposal_id;
use nym_validator_client::nyxd::cosmwasm_client::ToSingletonContractData;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{NymApiClient, ValidatorClientError};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

const CRED_SENDING_INTERVAL: u64 = 300;

#[derive(Clone)]
pub struct PendingCredential {
    pub credential: CredentialSpendingRequest,
    pub address: AccountId,
    pub proposal_id: Option<u64>,
}

impl PendingCredential {
    pub fn new(credential: CredentialSpendingRequest, address: AccountId) -> Self {
        PendingCredential {
            credential,
            address,
            proposal_id: None,
        }
    }
}

pub(crate) struct CredentialSender<St: Storage> {
    cred_receiver: UnboundedReceiver<PendingCredential>,
    storage: St,
    shared_state: SharedState,
}

impl<St> CredentialSender<St>
where
    St: Storage + 'static,
{
    pub(crate) fn new(
        cred_receiver: UnboundedReceiver<PendingCredential>,
        storage: St,
        shared_state: SharedState,
    ) -> Self {
        CredentialSender {
            cred_receiver,
            storage,
            shared_state,
        }
    }

    // async fn create_proposal(
    //     &self,
    //     pending: &mut PendingCredential,
    // ) -> Result<(), RequestHandlingError> {
    //     if pending.proposal_id.is_some() {
    //         log::trace!("Proposal already created");
    //         return Ok(());
    //     }
    //     let res = self
    //         .shared_state
    //         .start_tx()
    //         .await
    //         .prepare_credential(
    //             pending.credential.data.serial_number_b58(),
    //             pending.address.to_string(),
    //             None,
    //         )
    //         .await?;
    //     let proposal_id = find_proposal_id(&res.logs)?;
    //
    //     let proposal = self
    //         .shared_state
    //         .start_query()
    //         .await
    //         .query_proposal(proposal_id)
    //         .await?;
    //     if !pending
    //         .credential
    //         .matches_serial_number(&proposal.description)?
    //     {
    //         return Err(RequestHandlingError::ProposalIdError {
    //             reason: String::from("proposal has different serial number"),
    //         });
    //     }
    //     pending.proposal_id = Some(proposal_id);
    //     Ok(())
    // }
    //
    // async fn send_credential(pending: &mut PendingCredential) -> Result<(), RequestHandlingError> {
    //     let Some(proposal_id) = pending.proposal_id else {
    //         return Err(RequestHandlingError::ProposalIdError {
    //             reason: "proposal id is absent".to_string(),
    //         });
    //     };
    //
    //     let request = VerifyEcashCredentialBody::new(
    //         pending.credential.data.clone(),
    //         pending.address.clone(),
    //         proposal_id,
    //     );
    //     let mut failed_api = Vec::new();
    //     for client in &pending.api_clients {
    //         match client.verify_offline_credential(&request).await {
    //             Ok(response) => {
    //                 //Even if credential isn't accepted, we did contact the validator and resubmitting the same credential won't change anything. We can consider the sending as done
    //                 if response != VerifyEcashCredentialResponse::Accepted {
    //                     log::debug!(
    //                         "Validator {} didn't accept the credential - Reason : {}",
    //                         client.nym_api.current_url(),
    //                         response
    //                     );
    //                 }
    //             }
    //             Err(e) => {
    //                 log::warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", client.nym_api.current_url(), e);
    //                 failed_api.push(client.clone());
    //             }
    //         }
    //     }
    //     pending.api_clients = failed_api;
    //     if pending.api_clients.is_empty() {
    //         Ok(())
    //     } else {
    //         Err(RequestHandlingError::InternalError)
    //     }
    // }

    // the argument is temporary as we'll be reading from the storage
    async fn create_redemption_proposal(
        &self,
        pending: &PendingCredential,
    ) -> Result<u64, RequestHandlingError> {
        let res = self
            .shared_state
            .start_tx()
            .await
            .request_ticket_redemption(pending.credential.data.serial_number_b58(), 1, None)
            .await?;
        let proposal_id = res.parse_singleton_u64_contract_data()?;

        Ok(proposal_id)
    }

    async fn handle_credential_inner(
        &mut self,
        cred: PendingCredential,
    ) -> Result<(), RequestHandlingError> {
        let api_clients = self
            .shared_state
            .api_clients(cred.credential.data.epoch_id)
            .await?;

        let serial_number = cred.credential.serial_number_bs58();

        let request = VerifyEcashTicketBody {
            // TODO: redundant clone
            credential: cred.credential.data.clone(),
            gateway_cosmos_addr: self.shared_state.address.clone(),
        };

        let mut failed_apis = Arc::new(Mutex::new(Vec::new()));
        let mut rejected_apis = Arc::new(Mutex::new(Vec::new()));

        // TODO: save to db in case we crash

        // TODO: experiment with the value, but I reckon we can just pump it up quite high
        futures::stream::iter(api_clients.deref())
            .for_each_concurrent(32, |coconut_client| async {
                let name = coconut_client.to_string();

                match coconut_client
                    .api_client
                    .verify_ecash_ticket(&request)
                    .await
                {
                    Ok(res) => match res.verified {
                        Ok(_) => debug!("api {name} has accepted our ticket"),
                        Err(rejection) => {
                            warn!("api {name} has rejected our ticket because: {rejection}");
                            rejected_apis.lock().await.push(coconut_client.node_id)
                        }
                    },
                    Err(err) => {
                        error!("api {name} could not verify our ticket: {err}");
                        failed_apis.lock().await.push(coconut_client.node_id);
                    }
                }
            })
            .await;

        // TEMP:
        // safety the futures have completed so we MUST have the only arc reference
        #[allow(clippy::unwrap_used)]
        let failed_apis = Arc::into_inner(failed_apis).unwrap().into_inner();
        #[allow(clippy::unwrap_used)]
        let rejected_apis = Arc::into_inner(rejected_apis).unwrap().into_inner();

        if !rejected_apis.is_empty() {
            todo!("api rejection")
        }

        // TODO: we need 67%
        if failed_apis.len() > 2 {
            todo!("failed apis")
        }

        // temp block before split
        {
            warn!("temporary immediate redemption");

            let mut failed_apis = Arc::new(Mutex::new(Vec::new()));
            let mut rejected_apis = Arc::new(Mutex::new(Vec::new()));

            let sn = vec![serial_number];
            let digest = BatchRedeemTicketsBody::make_digest(sn.clone());
            let proposal_id = self.create_redemption_proposal(&cred).await?;

            let request = BatchRedeemTicketsBody::new(
                digest,
                proposal_id,
                sn,
                self.shared_state.address.to_string(),
            );

            futures::stream::iter(api_clients.deref())
                .for_each_concurrent(32, |coconut_client| async {
                    let name = coconut_client.to_string();

                    match coconut_client
                        .api_client
                        .batch_redeem_ecash_tickets(&request)
                        .await
                    {
                        Ok(res) => {
                            if res.proposal_accepted {
                                debug!("api {name} has accepted our redemption proposal");
                            } else {
                                warn!("api {name} has rejected our redemption proposal");
                                rejected_apis.lock().await.push(coconut_client.node_id)
                            }
                        }
                        Err(err) => {
                            error!("api {name} could not inspect our redemption proposal: {err}");
                            failed_apis.lock().await.push(coconut_client.node_id);
                        }
                    }
                })
                .await;

            // safety the futures have completed so we MUST have the only arc reference
            #[allow(clippy::unwrap_used)]
            let failed_apis = Arc::into_inner(failed_apis).unwrap().into_inner();
            #[allow(clippy::unwrap_used)]
            let rejected_apis = Arc::into_inner(rejected_apis).unwrap().into_inner();

            if !rejected_apis.is_empty() {
                todo!("api rejection")
            }

            // TODO: we need 67%
            if failed_apis.len() > 2 {
                todo!("failed apis")
            }
        }

        return Ok(());

        // TODO: another checkpoint

        // TODO: error handling here
        // persist ids of nodes we have to retry and do it on a timer

        // TODO: move to another timer/handler. this is just to have _something_ working end-to-end again

        todo!()
    }

    async fn handle_credential(&mut self, mut pending: PendingCredential) {
        // attempt to send for verification
        if let Err(err) = self.handle_credential_inner(pending).await {
            error!("credential handling failure: {err}")
        }

        // if self.create_proposal(&mut pending).await.is_err()
        //     || Self::send_credential(&mut pending).await.is_err()
        // {
        //     //failed to send, store credential
        //     if let Err(err) = self.storage.insert_pending_credential(pending).await {
        //         log::error!("Failed to store pending credential - {:?}", err);
        //     };
        // }
    }

    async fn try_empty_pending(&mut self) {
        error!("unimplemented");
        // log::debug!("Trying to send unsent payments");
        // let pending = match self.storage.get_all_pending_credential().await {
        //     Err(err) => {
        //         log::error!("Failed to retrieve pending credential - {:?}", err);
        //         return;
        //     }
        //     Ok(res) => res,
        // };
        //
        // for (id, mut pending) in pending {
        //     if self.create_proposal(&mut pending).await.is_ok() {
        //         //send successful, remove credential from storage
        //         if let Err(err) = self.storage.remove_pending_credential(id).await {
        //             log::error!("Failed to remove pending credential - {:?}", err);
        //         }
        //         if Self::send_credential(&mut pending).await.is_err() {
        //             //we didn't send to everyone one
        //             if let Err(err) = self.storage.insert_pending_credential(pending).await {
        //                 log::error!("Failed to store pending credential - {:?}", err);
        //             };
        //         }
        //     }
        // }
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

    pub(crate) fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
