// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config;
use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::types::{CredentialIssuer, MonitoringResults};
use crate::rewarder::nyxd_client::NyxdClient;
use bip39::rand::prelude::SliceRandom;
use bip39::rand::thread_rng;
use nym_coconut::{
    hash_to_scalar, verify_partial_blind_signature, Base58, G1Projective, Parameters,
    VerificationKey,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use nym_task::TaskClient;
use nym_validator_client::nym_api;
use nym_validator_client::nym_api::{IssuedCredentialBody, NymApiClientExt};
use nym_validator_client::nyxd::Hash;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

pub(crate) fn bandwidth_voucher_params() -> &'static Parameters {
    static BANDWIDTH_CREDENTIAL_PARAMS: OnceLock<Parameters> = OnceLock::new();
    BANDWIDTH_CREDENTIAL_PARAMS.get_or_init(|| BandwidthVoucher::default_parameters())
}

pub struct CredentialIssuanceMonitor {
    nyxd_client: NyxdClient,
    monitoring_results: MonitoringResults,
    config: config::IssuanceMonitor,
    // map of validator address -> transaction hash -> issued credential
    // (ideally we'd have hashed the AccountId directly, but it doesn't implement `Hash`)
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

    // TODO: currently we can't obtain public key of the runner in order to verify the signature
    async fn validate_issued_credential(
        &mut self,
        runner: String,
        credential_id: i64,
        issued_credential: IssuedCredentialBody,
        vk: &VerificationKey,
    ) -> Result<(), NymRewarderError> {
        warn!("unimplemented: public key sharing mechanism");
        // let plaintext = issued_credential.credential.signable_plaintext();
        // if !operator_public_key.verify(&plaintext) {
        // ...
        // }
        let tx_hash = issued_credential.credential.tx_hash;

        // check if we've seen this tx hash before
        // TODO: we should persist them in the database in case we crash
        if let Some(known_runner) = self.seen_deposits.get_mut(&runner) {
            if let Some(&used) = known_runner.get(&tx_hash) {
                return if used != credential_id {
                    Err(NymRewarderError::DuplicateDepositHash {
                        tx_hash,
                        first: used,
                        other: credential_id,
                    })
                } else {
                    debug!("we have already verified this credential before");
                    Ok(())
                };
            } else {
                known_runner.insert(tx_hash, credential_id);
            }
        }

        // check if this deposit even exists
        let (deposit_value, deposit_info) = self
            .nyxd_client
            .get_deposit_transaction_attributes(tx_hash)
            .await?;

        // check if the deposit values match
        let credential_value = issued_credential.credential.public_attributes.get(0);
        let credential_info = issued_credential.credential.public_attributes.get(1);

        if credential_value != Some(&deposit_value) {
            return Err(NymRewarderError::InconsistentDepositValue {
                tx_hash,
                request: credential_value.cloned(),
                on_chain: deposit_value,
            });
        }

        if credential_info != Some(&deposit_info) {
            return Err(NymRewarderError::InconsistentDepositInfo {
                tx_hash,
                request: credential_info.cloned(),
                on_chain: deposit_info,
            });
        }

        let public_attributes = issued_credential
            .credential
            .public_attributes
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();
        #[allow(clippy::map_identity)]
        let attributes_refs = public_attributes.iter().collect::<Vec<_>>();

        let mut public_attribute_commitments = Vec::with_capacity(
            issued_credential
                .credential
                .bs58_encoded_private_attributes_commitments
                .len(),
        );

        for raw_cm in issued_credential
            .credential
            .bs58_encoded_private_attributes_commitments
        {
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
            bandwidth_voucher_params(),
            &public_attribute_commitments,
            &attributes_refs,
            &issued_credential.credential.blinded_partial_credential,
            vk,
        ) {
            return Err(NymRewarderError::BlindVerificationFailure);
        }

        Ok(())
    }

    async fn check_issuer(
        &mut self,
        epoch_id: EpochId,
        issuer: CredentialIssuer,
    ) -> Result<(), NymRewarderError> {
        let url = match issuer.api_runner.parse() {
            Ok(url) => url,
            Err(source) => {
                return Err(NymRewarderError::MalformedApiUrl {
                    raw: issuer.api_runner,
                    runner_account: issuer.operator_account,
                    source,
                })
            }
        };

        let api_client = nym_api::Client::new(url, None);

        let epoch_credentials = api_client.epoch_credentials(epoch_id).await?;
        let Some(first_id) = epoch_credentials.first_epoch_credential_id else {
            // no point in doing anything more - if they haven't issued anything, there's nothing to verify
            debug!(
                "{} hasn't issued any credentials this epoch",
                issuer.operator_account
            );
            return Ok(());
        };

        let credential_range: Vec<_> =
            (first_id..first_id + epoch_credentials.total_issued as i64).collect();
        let issued = credential_range.len();

        let sampled = if issued <= self.config.min_validate_per_issuer as usize {
            credential_range
        } else {
            let mut rng = thread_rng();
            let sample_size = (issued as f32 * self.config.sampling_rate) as usize;
            credential_range
                .choose_multiple(&mut rng, sample_size)
                .copied()
                .collect::<Vec<_>>()
        };
        let request_size = sampled.len();

        let credentials = api_client.issued_credentials(sampled).await?;
        if credentials.credentials.len() != request_size {
            // TODO: we need some signatures here to actually show the validator is cheating
            return Err(NymRewarderError::IncompleteRequest {
                runner_account: issuer.operator_account,
                requested: request_size,
                received: credentials.credentials.len(),
            });
        }

        for (id, credential) in credentials.credentials {
            // TODO: insert the failure information, alongside the signature, to the evidence db
            if let Err(err) = self
                .validate_issued_credential(
                    issuer.operator_account.to_string(),
                    id,
                    credential,
                    &issuer.verification_key,
                )
                .await
            {
                error!(
                    "failed to verify credential {id} from {}!!: {err}",
                    issuer.operator_account
                );
                return Err(err);
            }
        }

        Ok(())
    }

    async fn check_issuers(&mut self) -> Result<(), NymRewarderError> {
        let epoch = self.nyxd_client.dkg_epoch().await?;
        let issuers = self
            .nyxd_client
            .get_credential_issuers(epoch.epoch_id)
            .await?;

        for issuer in issuers {
            let address = issuer.operator_account.clone();
            // we could parallelize it, but we're running the test so infrequently (relatively speaking)
            // that doing it sequentially is fine
            if let Err(err) = self.check_issuer(epoch.epoch_id, issuer).await {
                // TODO: insert info to the db
                error!("failed to check credential issuance of {address}: {err}")
            }
        }
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
