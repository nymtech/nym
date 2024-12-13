// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Error;
use credential_sender::CredentialHandler;
use credential_sender::CredentialHandlerConfig;
use error::EcashTicketError;
use futures::channel::mpsc::{self, UnboundedSender};
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::{ClientTicket, CompactEcashError, NymPayInfo, VerificationKeyAuth};
use nym_gateway_storage::Storage;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use state::SharedState;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLockReadGuard};
use tracing::error;

pub mod credential_sender;
pub mod error;
mod helpers;
mod state;

pub const TIME_RANGE_SEC: i64 = 30;

pub struct EcashManager<S> {
    shared_state: SharedState<S>,

    pk_bytes: [u8; 32], // bytes representation of a pub key representing the verifier
    pay_infos: Mutex<Vec<NymPayInfo>>,
    cred_sender: UnboundedSender<ClientTicket>,
}

impl<S> EcashManager<S>
where
    S: Storage + Clone + 'static,
{
    pub async fn new(
        credential_handler_cfg: CredentialHandlerConfig,
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        pk_bytes: [u8; 32],
        shutdown: nym_task::TaskClient,
        storage: S,
    ) -> Result<Self, Error> {
        let shared_state = SharedState::new(nyxd_client, storage).await?;

        let (cred_sender, cred_receiver) = mpsc::unbounded();

        let cs =
            CredentialHandler::new(credential_handler_cfg, cred_receiver, shared_state.clone())
                .await?;
        cs.start(shutdown);

        Ok(EcashManager {
            shared_state,
            pk_bytes,
            pay_infos: Default::default(),
            cred_sender,
        })
    }

    pub async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>, EcashTicketError> {
        self.shared_state.verification_key(epoch_id).await
    }

    pub fn storage(&self) -> &S {
        &self.shared_state.storage
    }

    //Check for duplicate pay_info, then check the payment, then insert pay_info if everything succeeded
    pub async fn check_payment(
        &self,
        credential: &CredentialSpendingData,
        aggregated_verification_key: &VerificationKeyAuth,
    ) -> Result<(), EcashTicketError> {
        let insert_index = self.verify_pay_info(credential.pay_info.into()).await?;

        credential
            .verify(aggregated_verification_key)
            .map_err(|err| match err {
                CompactEcashError::ExpirationDateSignatureValidity => {
                    EcashTicketError::MalformedTicketInvalidDateSignatures
                }
                _ => EcashTicketError::MalformedTicket,
            })?;

        self.insert_pay_info(credential.pay_info.into(), insert_index)
            .await
    }

    pub async fn verify_pay_info(&self, pay_info: NymPayInfo) -> Result<usize, EcashTicketError> {
        //Public key check
        if pay_info.pk() != self.pk_bytes {
            return Err(EcashTicketError::InvalidPayInfoPublicKey);
        }

        //Timestamp range check
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let tmin = timestamp - TIME_RANGE_SEC;
        let tmax = timestamp + TIME_RANGE_SEC;
        if pay_info.timestamp() > tmax || pay_info.timestamp() < tmin {
            return Err(EcashTicketError::InvalidPayInfoTimestamp);
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
                    return Err(EcashTicketError::DuplicatePayInfo);
                }
                //tbh, I don't expect ending up here if all parties are honest
                //binary search returns an arbitrary match, so we have to check for potential multiple matches
                let mut i = index as i64;
                while i >= 0 && inner[i as usize].timestamp() == pay_info.timestamp() {
                    if inner[i as usize] == pay_info {
                        return Err(EcashTicketError::DuplicatePayInfo);
                    }
                    i -= 1;
                }

                let mut i = index + 1;
                while i < inner.len() && inner[i].timestamp() == pay_info.timestamp() {
                    if inner[i] == pay_info {
                        return Err(EcashTicketError::DuplicatePayInfo);
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
    ) -> Result<(), EcashTicketError> {
        let mut inner = self.pay_infos.lock().await;
        if index > inner.len() {
            inner.push(pay_info);
            return Ok(());
        }
        inner.insert(index, pay_info);
        Ok(())
    }

    pub fn async_verify(&self, ticket: ClientTicket) {
        // TODO: I guess do something for shutdowns
        let _ = self
            .cred_sender
            .unbounded_send(ticket)
            .inspect_err(|_| error!("failed to send the client ticket for verification task"));
    }
}
