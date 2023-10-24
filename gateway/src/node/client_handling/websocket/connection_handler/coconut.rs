// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::authenticated::RequestHandlingError;
use chrono::Utc;
use log::*;
use nym_compact_ecash::scheme::EcashCredential;
use nym_compact_ecash::{PayInfo, VerificationKeyAuth};
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::{
    nyxd::{
        contract_traits::{DkgQueryClient, MultisigQueryClient, MultisigSigningClient},
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
        Coin, Fee,
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

const ONE_HOUR_SEC: u64 = 3600;
const MAX_FEEGRANT_UNYM: u128 = 10000;
const TIME_RANGE_SEC: i64 = 30;

pub(crate) struct EcashVerifier {
    nyxd_client: DirectSigningHttpRpcNyxdClient,
    pay_infos: Arc<Mutex<Vec<PayInfo>>>,
}

impl EcashVerifier {
    pub fn new(nyxd_client: DirectSigningHttpRpcNyxdClient) -> Self {
        EcashVerifier {
            nyxd_client,
            pay_infos: Arc::new(Mutex::new(Vec::new())),
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
                &credential.params(),
                aggregated_verification_key,
                &credential.pay_info(),
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
        //SW : TODO : implement Public key check (once an actual public key exist)

        let mut inner = self
            .pay_infos
            .lock()
            .map_err(|_| RequestHandlingError::InternalError)?;
        //Timestamp range check
        let timestamp = Utc::now().timestamp();
        let tmin = timestamp - TIME_RANGE_SEC;
        let tmax = timestamp + TIME_RANGE_SEC;
        if pay_info.timestamp() > tmax || pay_info.timestamp() < tmin {
            return Err(RequestHandlingError::InvalidBandwidthCredential(
                String::from("credential failed to verify on gateway - invalid timestamp"),
            ));
        }

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
        api_clients: Vec<CoconutApiClient>,
        credential: EcashCredential,
    ) -> Result<(), RequestHandlingError> {
        let req = nym_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nyxd_client.address(),
        );

        for client in api_clients {
            let ret = client.api_client.verify_bandwidth_credential(&req).await;

            match ret {
                Ok(res) => {
                    if !res.verification_result {
                        debug!(
                            "Validator {} didn't accept the credential.",
                            client.api_client.nym_api.current_url()
                        );
                    }
                }
                Err(e) => {
                    warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", client.api_client.nym_api.current_url(), e);
                }
            }
        }

        Ok(())
    }

    // pub async fn release_funds(
    //     &self,
    //     api_clients: Vec<CoconutApiClient>,
    //     credential: &nym_coconut_interface::Credential,
    // ) -> Result<(), RequestHandlingError> {
    //     // Use a custom multiplier for revoke, as the default one (1.3)
    //     // isn't enough
    //     let revoke_fee = Some(Fee::Auto(Some(1.5)));

    //     let res = self
    //         .nyxd_client
    //         .spend_credential(
    //             Coin::new(
    //                 credential.voucher_value().into(),
    //                 self.mix_denom_base.clone(),
    //             ),
    //             credential.blinded_serial_number(),
    //             self.nyxd_client.address().to_string(),
    //             None,
    //         )
    //         .await?;
    //     let proposal_id = find_attribute(&res.logs, "wasm", BANDWIDTH_PROPOSAL_ID)
    //         .ok_or(RequestHandlingError::ProposalIdError {
    //             reason: String::from("proposal id not found"),
    //         })?
    //         .value
    //         .parse::<u64>()
    //         .map_err(|_| RequestHandlingError::ProposalIdError {
    //             reason: String::from("proposal id could not be parsed to u64"),
    //         })?;

    //     let proposal = self.nyxd_client.query_proposal(proposal_id).await?;
    //     if !credential.has_blinded_serial_number(&proposal.description)? {
    //         return Err(RequestHandlingError::ProposalIdError {
    //             reason: String::from("proposal has different serial number"),
    //         });
    //     }

    //     let req = nym_api_requests::coconut::VerifyCredentialBody::new(
    //         credential.clone(),
    //         //proposal_id,
    //         self.nyxd_client.address().clone(),
    //     );
    //     for client in api_clients {
    //         self.nyxd_client
    //             .grant_allowance(
    //                 &client.cosmos_address,
    //                 vec![Coin::new(MAX_FEEGRANT_UNYM, self.mix_denom_base.clone())],
    //                 SystemTime::now().checked_add(Duration::from_secs(ONE_HOUR_SEC)),
    //                 // It would be nice to be able to filter deeper, but for now only the msg type filter is avaialable
    //                 vec![String::from("/cosmwasm.wasm.v1.MsgExecuteContract")],
    //                 "Create allowance to vote the release of funds".to_string(),
    //                 None,
    //             )
    //             .await?;
    //         let ret = client.api_client.verify_bandwidth_credential(&req).await;
    //         self.nyxd_client
    //             .revoke_allowance(
    //                 &client.cosmos_address,
    //                 "Cleanup the previous allowance for releasing funds".to_string(),
    //                 revoke_fee.clone(),
    //             )
    //             .await?;
    //         match ret {
    //             Ok(res) => {
    //                 if !res.verification_result {
    //                     debug!("Validator {} didn't accept the credential. It will probably vote No on the spending proposal", client.api_client.nym_api.current_url());
    //                 }
    //             }
    //             Err(e) => {
    //                 warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", client.api_client.nym_api.current_url(), e);
    //             }
    //         }
    //     }

    //     self.nyxd_client.execute_proposal(proposal_id, None).await?;

    //     Ok(())
    // }
}
