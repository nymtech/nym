// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::authenticated::RequestHandlingError;
use crate::node::client_handling::websocket::connection_handler::ecash::state::SharedState;
use crate::node::storage::Storage;
use crate::GatewayError;
use credential_sender::CredentialSender;
use double_spending::DoubleSpendingDetector;
use futures::channel::mpsc::{self, UnboundedSender};
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::{CompactEcashError, NymPayInfo, VerificationKeyAuth};
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::{CoconutApiClient, DirectSigningHttpRpcNyxdClient};
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLockReadGuard};

pub use credential_sender::PendingCredential;

mod credential_sender;
mod double_spending;
mod state;

const TIME_RANGE_SEC: i64 = 30;

pub struct EcashManager {
    shared_state: SharedState,

    pk_bytes: [u8; 32], // bytes representation of a pub key representing the verifier
    pay_infos: Mutex<Vec<NymPayInfo>>,
    cred_sender: UnboundedSender<PendingCredential>,
    double_spend_detector: DoubleSpendingDetector,
}

impl EcashManager {
    pub async fn new<St: Storage + 'static>(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        pk_bytes: [u8; 32],
        shutdown: nym_task::TaskClient,
        storage: St,
    ) -> Result<Self, GatewayError> {
        let shared_state = SharedState::new(nyxd_client).await?;

        let double_spend_detector = DoubleSpendingDetector::new(shared_state.clone());
        double_spend_detector.clone().start(shutdown.clone());

        let (cred_sender, cred_receiver) = mpsc::unbounded();

        let cs = CredentialSender::new(cred_receiver, storage, shared_state.clone());
        cs.start(shutdown);

        Ok(EcashManager {
            shared_state,
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
        self.shared_state.api_clients(epoch_id).await
    }

    pub async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>, RequestHandlingError> {
        self.shared_state.verification_key(epoch_id).await
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
                    RequestHandlingError::MalformedCredentialInvalidDateSignatures
                }
                _ => RequestHandlingError::MalformedCredential,
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
        self.double_spend_detector.check(serial_number_bs58).await
    }

    pub fn post_credential(
        &self,
        api_clients: &[CoconutApiClient],
        credential: CredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        self.cred_sender
            .unbounded_send(PendingCredential::new(
                credential.clone(),
                self.shared_state.address.clone(),
                api_clients
                    .iter()
                    .map(|client| client.api_client.clone())
                    .collect(),
            ))
            .map_err(|_| RequestHandlingError::InternalError)
    }
}
