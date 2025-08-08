// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Error;
use async_trait::async_trait;
use credential_sender::CredentialHandler;
use credential_sender::CredentialHandlerConfig;
use error::EcashTicketError;
use futures::channel::mpsc::{self, UnboundedSender};
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::{ClientTicket, CompactEcashError, NymPayInfo, VerificationKeyAuth};
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_gateway_storage::GatewayStorage;
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
pub mod traits;

pub const TIME_RANGE_SEC: i64 = 30;

pub struct EcashManager {
    shared_state: SharedState,

    pk_bytes: [u8; 32], // bytes representation of a pub key representing the verifier
    pay_infos: Mutex<Vec<NymPayInfo>>,
    cred_sender: UnboundedSender<ClientTicket>,
}

#[async_trait]
impl traits::EcashManager for EcashManager {
    async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, EcashTicketError> {
        self.shared_state.verification_key(epoch_id).await
    }

    fn storage(&self) -> Box<dyn BandwidthGatewayStorage + Send + Sync> {
        dyn_clone::clone_box(&*self.shared_state.storage)
    }

    //Check for duplicate pay_info, then check the payment, then insert pay_info if everything succeeded
    async fn check_payment(
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

    fn async_verify(&self, ticket: ClientTicket) {
        // TODO: I guess do something for shutdowns
        let _ = self
            .cred_sender
            .unbounded_send(ticket)
            .inspect_err(|_| error!("failed to send the client ticket for verification task"));
    }
}

impl EcashManager {
    pub async fn new(
        credential_handler_cfg: CredentialHandlerConfig,
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        pk_bytes: [u8; 32],
        shutdown: nym_task::TaskClient,
        storage: GatewayStorage,
    ) -> Result<Self, Error> {
        let shared_state = SharedState::new(nyxd_client, Box::new(storage)).await?;

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
}

pub struct MockEcashManager {
    verfication_key: tokio::sync::RwLock<VerificationKeyAuth>,
    storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,
}

impl MockEcashManager {
    pub fn new(storage: Box<dyn BandwidthGatewayStorage + Send + Sync>) -> Self {
        Self {
            verfication_key: tokio::sync::RwLock::new(
                VerificationKeyAuth::from_bytes(&[
                    129, 187, 76, 12, 1, 51, 46, 26, 132, 205, 148, 109, 140, 131, 50, 119, 45,
                    128, 51, 218, 106, 70, 181, 74, 244, 38, 162, 62, 42, 12, 5, 100, 7, 136, 32,
                    155, 18, 219, 195, 182, 3, 56, 168, 16, 93, 154, 249, 230, 16, 202, 90, 134,
                    246, 25, 98, 6, 175, 215, 188, 239, 71, 84, 66, 1, 43, 66, 197, 180, 216, 80,
                    55, 185, 140, 216, 14, 48, 244, 214, 20, 68, 106, 41, 48, 252, 188, 181, 231,
                    170, 23, 211, 215, 12, 91, 147, 47, 7, 4, 0, 0, 0, 0, 0, 0, 0, 174, 31, 237,
                    215, 159, 183, 71, 125, 90, 147, 84, 78, 49, 216, 66, 232, 92, 206, 41, 230,
                    239, 209, 211, 166, 131, 190, 148, 36, 225, 194, 146, 6, 120, 34, 194, 5, 154,
                    155, 234, 41, 191, 119, 227, 51, 91, 128, 151, 240, 129, 208, 253, 171, 234,
                    170, 71, 139, 251, 78, 49, 35, 218, 16, 77, 150, 177, 204, 83, 210, 67, 147,
                    66, 162, 58, 25, 96, 168, 61, 180, 92, 21, 18, 78, 194, 98, 176, 123, 122, 176,
                    81, 150, 187, 20, 64, 69, 0, 134, 142, 3, 84, 108, 3, 55, 107, 111, 73, 31, 46,
                    51, 225, 248, 202, 173, 194, 24, 104, 96, 31, 61, 24, 140, 220, 31, 176, 200,
                    30, 217, 66, 58, 11, 181, 158, 196, 179, 199, 177, 7, 210, 4, 119, 142, 149,
                    59, 3, 186, 145, 27, 230, 125, 230, 246, 197, 196, 119, 70, 239, 115, 99, 215,
                    63, 205, 63, 74, 108, 201, 42, 226, 150, 137, 3, 157, 45, 25, 163, 54, 107,
                    153, 61, 141, 64, 207, 139, 41, 203, 39, 36, 97, 181, 72, 206, 235, 221, 178,
                    171, 60, 4, 6, 170, 181, 213, 10, 216, 53, 28, 32, 33, 41, 224, 60, 247, 206,
                    137, 108, 251, 229, 234, 112, 65, 145, 124, 212, 125, 116, 154, 114, 2, 125,
                    202, 24, 25, 196, 219, 104, 200, 131, 133, 180, 39, 21, 144, 204, 8, 151, 218,
                    99, 64, 209, 47, 5, 42, 13, 214, 139, 54, 112, 224, 53, 238, 250, 56, 42, 105,
                    15, 21, 238, 99, 225, 79, 121, 104, 155, 230, 243, 133, 47, 39, 147, 98, 45,
                    113, 137, 200, 102, 151, 122, 174, 9, 250, 17, 138, 191, 129, 202, 244, 107,
                    75, 48, 141, 136, 89, 168, 124, 88, 174, 251, 17, 35, 146, 88, 76, 134, 102,
                    105, 204, 16, 176, 214, 63, 13, 170, 225, 250, 112, 7, 237, 161, 160, 15, 71,
                    10, 130, 137, 69, 186, 64, 223, 188, 5, 5, 228, 57, 214, 134, 247, 20, 171,
                    140, 43, 230, 57, 29, 127, 136, 169, 80, 14, 137, 130, 200, 205, 222, 81, 143,
                    40, 77, 68, 197, 91, 142, 91, 84, 164, 15, 133, 242, 149, 255, 173, 201, 108,
                    208, 23, 188, 230, 158, 146, 54, 198, 52, 148, 123, 202, 52, 222, 50, 4, 62,
                    211, 208, 176, 61, 104, 151, 227, 192, 224, 200, 132, 53, 187, 240, 254, 150,
                    60, 30, 140, 11, 63, 71, 12, 30, 233, 255, 144, 250, 16, 81, 38, 33, 9, 185,
                    195, 214, 0, 119, 117, 94, 100, 103, 144, 10, 189, 65, 113, 114, 192, 11, 177,
                    214, 223, 218, 36, 139, 183, 2, 206, 247, 245, 88, 62, 231, 183, 50, 46, 95,
                    202, 152, 82, 244, 80, 173, 192, 147, 51, 248, 46, 181, 194, 205, 233, 67, 144,
                    155, 250, 142, 124, 71, 9, 136, 142, 88, 29, 99, 222, 43, 181, 172, 120, 187,
                    179, 172, 240, 231, 57, 236, 195, 158, 182, 203, 19, 49, 220, 180, 212, 101,
                    105, 239, 58, 215, 0, 50, 100, 172, 29, 236, 170, 108, 129, 150, 5, 64, 238,
                    59, 50, 4, 21, 131, 197, 142, 191, 76, 101, 140, 133, 112, 38, 235, 113, 203,
                    22, 161, 204, 84, 73, 125, 219, 70, 62, 67, 119, 52, 130, 208, 180, 231, 78,
                    141, 181, 13, 207, 196, 126, 159, 70, 34, 195, 70,
                ])
                .unwrap(),
            ),
            storage: dyn_clone::clone_box(&*storage),
        }
    }
}

#[async_trait]
impl traits::EcashManager for MockEcashManager {
    async fn verification_key(
        &self,
        _epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, EcashTicketError> {
        Ok(self.verfication_key.read().await)
    }

    fn storage(&self) -> Box<dyn BandwidthGatewayStorage + Send + Sync> {
        dyn_clone::clone_box(&*self.storage)
    }

    async fn check_payment(
        &self,
        _credential: &CredentialSpendingData,
        _aggregated_verification_key: &VerificationKeyAuth,
    ) -> Result<(), EcashTicketError> {
        Ok(())
    }

    fn async_verify(&self, _ticket: ClientTicket) {}
}
