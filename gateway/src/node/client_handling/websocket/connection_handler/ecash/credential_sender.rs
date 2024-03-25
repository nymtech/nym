// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::Storage;

use futures::channel::mpsc::UnboundedReceiver;
use futures::StreamExt;
use nym_api_requests::coconut::OfflineVerifyCredentialBody;

use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::NymApiClient;
use tokio::time::{interval, Duration};

const CRED_SENDING_INTERVAL: u64 = 300;

#[derive(Clone)]
pub struct PendingCredential {
    pub credential: CredentialSpendingRequest,
    pub address: AccountId,
    pub client: NymApiClient,
}

pub(crate) struct CredentialSender<St: Storage> {
    cred_receiver: UnboundedReceiver<PendingCredential>,
    storage: St,
}

impl<St> CredentialSender<St>
where
    St: Storage + 'static,
{
    pub(crate) fn new(cred_receiver: UnboundedReceiver<PendingCredential>, storage: St) -> Self {
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

    pub(crate) fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
