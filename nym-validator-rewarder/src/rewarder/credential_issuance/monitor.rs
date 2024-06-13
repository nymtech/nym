// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config;
use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::types::{
    CredentialIssuer, MonitoringResults, RawOperatorResult,
};
use crate::rewarder::helpers::api_client;
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use bip39::rand::prelude::SliceRandom;
use bip39::rand::thread_rng;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::scheme::withdrawal::verify_partial_blind_signature;
use nym_compact_ecash::{Attribute, Base58, G1Projective, VerificationKeyAuth};
use nym_crypto::asymmetric::ed25519;
use nym_task::TaskClient;
use nym_validator_client::nym_api::{IssuedCredential, IssuedCredentialBody, NymApiClientExt};
use std::cmp::max;
use tokio::time::interval;
use tracing::{debug, error, info, instrument, trace};

pub struct CredentialIssuanceMonitor {
    nyxd_client: NyxdClient,
    monitoring_results: MonitoringResults,
    config: config::IssuanceMonitor,
    storage: RewarderStorage,
}

impl CredentialIssuanceMonitor {
    pub fn new(
        config: config::IssuanceMonitor,
        nyxd_client: NyxdClient,
        storage: RewarderStorage,
        monitoring_results: MonitoringResults,
    ) -> CredentialIssuanceMonitor {
        CredentialIssuanceMonitor {
            config,
            storage,
            nyxd_client,
            monitoring_results,
        }
    }

    fn validate_credential_signature(
        &mut self,
        issued_credential: &IssuedCredentialBody,
        identity_key: &ed25519::PublicKey,
    ) -> Result<(), NymRewarderError> {
        let plaintext = issued_credential.credential.signable_plaintext();
        if identity_key
            .verify(plaintext, &issued_credential.signature)
            .is_err()
        {
            Err(NymRewarderError::SignatureVerificationFailure {
                credential_id: issued_credential.credential.id,
            })
        } else {
            Ok(())
        }
    }

    async fn check_deposit_reuse(
        &mut self,
        issuer_identity: &str,
        credential_info: &IssuedCredentialBody,
    ) -> Result<bool, NymRewarderError> {
        let credential_id = credential_info.credential.id;
        let deposit_id = credential_info.credential.deposit_id;
        let prior_id = self
            .storage
            .get_deposit_credential_id(issuer_identity.to_string(), deposit_id)
            .await?;

        match prior_id {
            None => Ok(false),
            Some(prior) => {
                if prior == credential_id {
                    debug!("we have already verified this credential before");
                    Ok(true)
                } else {
                    error!("double signing detected!! used deposit {deposit_id} for credentials {prior} and {credential_id}!!");
                    self.storage
                        .insert_double_signing_evidence(
                            issuer_identity.to_string(),
                            prior,
                            credential_info,
                        )
                        .await?;
                    Err(NymRewarderError::DuplicateDepositId {
                        deposit_id,
                        first: prior,
                        other: credential_id,
                    })
                }
            }
        }
    }

    async fn validate_deposit(
        &mut self,
        issued_credential: &IssuedCredentialBody,
    ) -> Result<(), NymRewarderError> {
        // check if this deposit even exists
        let deposit_id = issued_credential.credential.deposit_id;

        //not using value anymore, but it should still be there
        let _ = self.nyxd_client.get_deposit_details(deposit_id).await?;
        trace!("deposit exists");

        Ok(())
    }

    fn verify_credential(
        &mut self,
        vk: &VerificationKeyAuth,
        credential: &IssuedCredential,
    ) -> Result<(), NymRewarderError> {
        let public_attributes = [Attribute::from(
            credential.expiration_date.unix_timestamp() as u64
        )];

        #[allow(clippy::map_identity)]
        let attributes_refs = public_attributes.iter().collect::<Vec<_>>();

        let mut public_attribute_commitments =
            Vec::with_capacity(credential.bs58_encoded_private_attributes_commitments.len());

        for raw_cm in &credential.bs58_encoded_private_attributes_commitments {
            match G1Projective::try_from_bs58(raw_cm) {
                Ok(cm) => public_attribute_commitments.push(cm),
                Err(source) => {
                    return Err(NymRewarderError::MalformedCredentialCommitment {
                        raw: raw_cm.clone(),
                        source,
                    })
                }
            }
        }

        // actually do verify the credential now
        if !verify_partial_blind_signature(
            &public_attribute_commitments,
            &attributes_refs,
            &credential.blinded_partial_credential,
            vk,
        ) {
            return Err(NymRewarderError::BlindVerificationFailure);
        }
        trace!("credential correctly verifies");

        Ok(())
    }

    #[instrument(skip_all, fields(credential_id = %issued_credential.credential.id, deposit_id = %issued_credential.credential.deposit_id))]
    async fn validate_issued_credential(
        &mut self,
        issuer: &CredentialIssuer,
        issued_credential: &IssuedCredentialBody,
    ) -> Result<(), NymRewarderError> {
        // check if the issuer has actually signed that issued credential information
        self.validate_credential_signature(issued_credential, &issuer.public_key)?;
        let encoded_key = issuer.public_key.to_base58_string();

        // make sure the issuer is not using the same deposit for multiple credentials
        let already_checked = self
            .check_deposit_reuse(&encoded_key, issued_credential)
            .await?;
        if already_checked {
            return Ok(());
        }

        // check the correctness of the deposit itself
        self.validate_deposit(issued_credential).await?;

        // insert validated deposit info into the storage
        self.storage
            .insert_validated_deposit(encoded_key, issued_credential)
            .await?;

        // check if the partial credential correctly verifies
        self.verify_credential(&issuer.verification_key, &issued_credential.credential)?;

        Ok(())
    }

    fn sample_credential_ids(&self, first_id: i64, total_issued: i64) -> Vec<i64> {
        let credential_range: Vec<_> = (first_id..first_id + total_issued).collect();
        let issued = credential_range.len();

        let sampled = if issued <= self.config.min_validate_per_issuer {
            credential_range
        } else {
            let mut rng = thread_rng();
            let sample_size = max(
                self.config.min_validate_per_issuer,
                (issued as f64 * self.config.sampling_rate) as usize,
            );
            credential_range
                .choose_multiple(&mut rng, sample_size)
                .copied()
                .collect::<Vec<_>>()
        };

        sampled
    }

    #[instrument(skip(self, issuer, epoch_id), fields(dkg_epoch = epoch_id, issuer = %issuer.operator_account, url = issuer.api_runner), err(Display))]
    async fn check_issuer(
        &mut self,
        epoch_id: EpochId,
        issuer: &CredentialIssuer,
    ) -> Result<RawOperatorResult, NymRewarderError> {
        info!("checking the issuer's credentials...");
        debug!("checking the issuer's credentials...");

        let api_client = api_client(issuer)?;

        let epoch_credentials = api_client.epoch_credentials(epoch_id).await?;
        let whitelisted = self.config.whitelist.contains(&issuer.operator_account);

        let Some(first_id) = epoch_credentials.first_epoch_credential_id else {
            // no point in doing anything more - if they haven't issued anything, there's nothing to verify
            debug!("no credentials issued this epoch",);
            return Ok(RawOperatorResult::new_empty(
                issuer.operator_account.clone(),
                issuer.api_runner.clone(),
                whitelisted,
            ));
        };
        trace!("issued credentials: {epoch_credentials:?}");

        let sampled = self.sample_credential_ids(first_id, epoch_credentials.total_issued as i64);
        let request_size = sampled.len();

        trace!("sampled credentials to validate: {sampled:?}");

        let credentials = api_client.issued_credentials(sampled.clone()).await?;
        if credentials.credentials.len() != request_size {
            error!("received an incomplete credential request! the issuer **MIGHT** be cheating!! but we're lacking sufficient signatures to be certain");
            return Err(NymRewarderError::IncompleteRequest {
                runner_account: issuer.operator_account.clone(),
                requested: request_size,
                received: credentials.credentials.len(),
            });
        }

        for (id, credential) in credentials.credentials {
            trace!("checking credential {id}...");
            if let Err(err) = self.validate_issued_credential(issuer, &credential).await {
                error!(
                    "failed to validate credential {id} from {} ({})!!: {err}",
                    issuer.public_key, issuer.operator_account
                );
                self.storage
                    .insert_issuance_foul_play_evidence(issuer, &credential, err.to_string())
                    .await?;
                return Err(err);
            }
        }

        Ok(RawOperatorResult {
            operator_account: issuer.operator_account.clone(),
            api_runner: issuer.api_runner.clone(),
            whitelisted,
            issued_credentials: epoch_credentials.total_issued,
            validated_credentials: sampled,
        })
    }

    async fn check_issuers(&mut self) -> Result<(), NymRewarderError> {
        info!("checking credential issuers");
        let epoch = self.nyxd_client.dkg_epoch().await?;
        let issuers = self
            .nyxd_client
            .get_credential_issuers(epoch.epoch_id)
            .await?;

        let mut results = Vec::with_capacity(issuers.len());

        for issuer in issuers {
            // we could parallelize it, but we're running the test so infrequently (relatively speaking)
            // that doing it sequentially is fine
            match self.check_issuer(epoch.epoch_id, &issuer).await {
                Ok(res) => results.push(res),
                Err(err) => {
                    let address = &issuer.operator_account;
                    error!("failed to check credential issuance of {address}: {err}");
                    self.storage
                        .insert_issuance_validation_failure_info(&issuer, err.to_string())
                        .await?;
                }
            }
        }

        self.monitoring_results
            .append_run_results(epoch.epoch_id as u32, results)
            .await;

        Ok(())
    }

    pub async fn run(&mut self, mut task_client: TaskClient) {
        info!("starting");
        let mut run_interval = interval(self.config.run_interval);

        while !task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = task_client.recv() => {
                    info!("received shutdown");
                    break
                }
                _ = run_interval.tick() => {
                    if let Err(err) = self.check_issuers().await {
                        error!("failed to perform credential issuance check: {err}")
                    }
                }
            }
        }
    }
}
