// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config;
use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::types::{
    CredentialIssuer, MonitoringResults, RawOperatorResult,
};
use crate::rewarder::helpers::api_client;
use crate::rewarder::nyxd_client::NyxdClient;
use bip39::rand::prelude::SliceRandom;
use bip39::rand::thread_rng;
use nym_coconut::{
    hash_to_scalar, verify_partial_blind_signature, Base58, G1Projective, VerificationKey,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_credentials::coconut::bandwidth::bandwidth_credential_params;
use nym_crypto::asymmetric::ed25519;
use nym_task::TaskClient;
use nym_validator_client::nym_api::{IssuedCredential, IssuedCredentialBody, NymApiClientExt};
use nym_validator_client::nyxd::Hash;
use std::cmp::max;
use std::collections::HashMap;
use tokio::time::interval;
use tracing::{debug, error, info, instrument, trace};

pub struct CredentialIssuanceMonitor {
    nyxd_client: NyxdClient,
    monitoring_results: MonitoringResults,
    config: config::IssuanceMonitor,

    // map of validator identity -> transaction hash -> issued credential
    seen_deposits: HashMap<String, HashMap<Hash, i64>>,
}

impl CredentialIssuanceMonitor {
    pub fn new(
        config: config::IssuanceMonitor,
        nyxd_client: NyxdClient,
        monitoring_results: MonitoringResults,
    ) -> CredentialIssuanceMonitor {
        CredentialIssuanceMonitor {
            config,
            nyxd_client,
            monitoring_results,
            seen_deposits: Default::default(),
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

    fn check_deposit_reuse(
        &mut self,
        issuer_identity: &str,
        credential_id: i64,
        deposit_tx: Hash,
    ) -> Result<bool, NymRewarderError> {
        // check if we've seen this tx hash before
        // TODO: we should persist them in the database in case we crash
        if let Some(known_issuer) = self.seen_deposits.get_mut(issuer_identity) {
            if let Some(&used) = known_issuer.get(&deposit_tx) {
                return if used != credential_id {
                    Err(NymRewarderError::DuplicateDepositHash {
                        tx_hash: deposit_tx,
                        first: used,
                        other: credential_id,
                    })
                } else {
                    debug!("we have already verified this credential before");
                    Ok(true)
                };
            } else {
                known_issuer.insert(deposit_tx, credential_id);
            }
        }
        Ok(false)
    }

    async fn validate_deposit(
        &mut self,
        issued_credential: &IssuedCredentialBody,
    ) -> Result<(), NymRewarderError> {
        // check if this deposit even exists
        let deposit_tx = issued_credential.credential.tx_hash;

        let (deposit_value, deposit_info) = self
            .nyxd_client
            .get_deposit_transaction_attributes(deposit_tx)
            .await?;
        trace!("deposit exists");

        // check if the deposit values match
        let credential_value = issued_credential.credential.public_attributes.first();
        let credential_info = issued_credential.credential.public_attributes.get(1);

        if credential_value != Some(&deposit_value) {
            return Err(NymRewarderError::InconsistentDepositValue {
                tx_hash: deposit_tx,
                request: credential_value.cloned(),
                on_chain: deposit_value,
            });
        }
        trace!("credential values matches the deposit");

        if credential_info != Some(&deposit_info) {
            return Err(NymRewarderError::InconsistentDepositInfo {
                tx_hash: deposit_tx,
                request: credential_info.cloned(),
                on_chain: deposit_info,
            });
        }
        trace!("credential info matches the deposit");
        Ok(())
    }

    fn verify_credential(
        &mut self,
        vk: &VerificationKey,
        credential: IssuedCredential,
    ) -> Result<(), NymRewarderError> {
        let public_attributes = credential
            .public_attributes
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();

        #[allow(clippy::map_identity)]
        let attributes_refs = public_attributes.iter().collect::<Vec<_>>();

        let mut public_attribute_commitments =
            Vec::with_capacity(credential.bs58_encoded_private_attributes_commitments.len());

        for raw_cm in credential.bs58_encoded_private_attributes_commitments {
            match G1Projective::try_from_bs58(&raw_cm) {
                Ok(cm) => public_attribute_commitments.push(cm),
                Err(source) => {
                    return Err(NymRewarderError::MalformedCredentialCommitment {
                        raw: raw_cm,
                        source,
                    })
                }
            }
        }

        // actually do verify the credential now
        if !verify_partial_blind_signature(
            bandwidth_credential_params(),
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

    #[instrument(skip_all, fields(credential_id = credential_id, deposit = %issued_credential.credential.tx_hash))]
    async fn validate_issued_credential(
        &mut self,
        issuer: &CredentialIssuer,
        credential_id: i64,
        issued_credential: IssuedCredentialBody,
    ) -> Result<(), NymRewarderError> {
        // check if the issuer has actually signed that issued credential information
        self.validate_credential_signature(&issued_credential, &issuer.public_key)?;
        let encoded_key = issuer.public_key.to_base58_string();

        let deposit_tx = issued_credential.credential.tx_hash;

        // make sure the issuer is not using the same deposit for multiple credentials
        let already_checked = self.check_deposit_reuse(&encoded_key, credential_id, deposit_tx)?;
        if already_checked {
            return Ok(());
        }

        // check the correctness of the deposit itself
        self.validate_deposit(&issued_credential).await?;

        // check if the partial credential correctly verifies
        self.verify_credential(&issuer.verification_key, issued_credential.credential)?;

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
        issuer: CredentialIssuer,
    ) -> Result<RawOperatorResult, NymRewarderError> {
        info!("checking the issuer's credentials...");
        debug!("checking the issuer's credentials...");

        let api_client = api_client(&issuer)?;

        let epoch_credentials = api_client.epoch_credentials(epoch_id).await?;
        let whitelisted = self.config.whitelist.contains(&issuer.operator_account);

        let Some(first_id) = epoch_credentials.first_epoch_credential_id else {
            // no point in doing anything more - if they haven't issued anything, there's nothing to verify
            debug!("no credentials issued this epoch",);
            return Ok(RawOperatorResult::new_empty(
                issuer.operator_account,
                issuer.api_runner,
                whitelisted,
            ));
        };
        trace!("issued credentials: {epoch_credentials:?}");

        let sampled = self.sample_credential_ids(first_id, epoch_credentials.total_issued as i64);
        let request_size = sampled.len();

        trace!("sampled credentials to validate: {sampled:?}");

        let credentials = api_client.issued_credentials(sampled.clone()).await?;
        if credentials.credentials.len() != request_size {
            // TODO: we need some signatures here to actually show the validator is cheating
            return Err(NymRewarderError::IncompleteRequest {
                runner_account: issuer.operator_account,
                requested: request_size,
                received: credentials.credentials.len(),
            });
        }

        for (id, credential) in credentials.credentials {
            trace!("checking credential {id}...");
            // TODO: insert the failure information, alongside the signature, to the evidence db
            if let Err(err) = self
                .validate_issued_credential(&issuer, id, credential)
                .await
            {
                error!(
                    "failed to verify credential {id} from {} ({})!!: {err}",
                    issuer.public_key, issuer.operator_account
                );
                return Err(err);
            }
        }

        Ok(RawOperatorResult {
            operator_account: issuer.operator_account,
            api_runner: issuer.api_runner,
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
            let address = issuer.operator_account.clone();
            // we could parallelize it, but we're running the test so infrequently (relatively speaking)
            // that doing it sequentially is fine
            match self.check_issuer(epoch.epoch_id, issuer).await {
                Ok(res) => results.push(res),
                Err(err) => {
                    // TODO: insert info to the db
                    error!("failed to check credential issuance of {address}: {err}")
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
