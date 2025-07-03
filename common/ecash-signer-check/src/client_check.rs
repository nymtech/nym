// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::chain_status::LocalChainStatus;
use crate::signing_status::SigningStatus;
use crate::{
    chain_status, signing_status, SignerInformation, SignerResult, SignerStatus, SignerTestResult,
};
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::BinaryBuildInformationOwned;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::ContractVKShare;
use nym_validator_client::EcashApiClient;
use std::time::Duration;
use tracing::{error, warn};

struct ClientUnderTest {
    api_client: EcashApiClient,
    build_info: Option<BinaryBuildInformationOwned>,
}

impl ClientUnderTest {
    pub(crate) fn new(api_client: EcashApiClient) -> Self {
        ClientUnderTest {
            api_client,
            build_info: None,
        }
    }

    pub(crate) async fn try_retrieve_build_information(&mut self) -> bool {
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.api_client.api_client.nym_api.build_information(),
        )
        .await
        {
            Ok(Ok(build_information)) => {
                self.build_info = Some(build_information);
                true
            }
            Ok(Err(err)) => {
                warn!("{}: failed to retrieve build information: {err}. the signer is most likely down", self.api_client);
                false
            }
            Err(_timeout) => {
                warn!(
                    "{}: timed out while attempting to retrieve build information",
                    self.api_client
                );
                false
            }
        }
    }

    pub(crate) fn version(&self) -> Option<semver::Version> {
        self.build_info.as_ref().and_then(|build_info| {
            build_info
                .build_version
                .parse()
                .inspect_err(|err| {
                    error!(
                        "ecash signer '{}' reports invalid version {}: {err}",
                        self.api_client, build_info.build_version
                    )
                })
                .ok()
        })
    }

    pub(crate) fn supports_signing_status_query(&self) -> bool {
        let Some(version) = self.version() else {
            return false;
        };
        version >= signing_status::MINIMUM_VERSION
    }

    pub(crate) fn supports_chain_status_query(&self) -> bool {
        let Some(version) = self.version() else {
            return false;
        };
        version >= chain_status::MINIMUM_VERSION
    }

    pub(crate) async fn check_local_chain(&self) -> LocalChainStatus {
        if !self.supports_chain_status_query() {
            return LocalChainStatus::Outdated;
        }

        match self.api_client.api_client.nym_api.get_chain_status().await {
            Ok(status) => LocalChainStatus::Reachable {
                response: Box::new(status),
            },
            Err(err) => {
                warn!(
                    "{}: failed to retrieve local chain status: {err}",
                    self.api_client
                );
                LocalChainStatus::Unreachable
            }
        }
    }

    pub(crate) async fn check_signing_status(&self) -> SigningStatus {
        if !self.supports_signing_status_query() {
            return SigningStatus::Outdated;
        }

        match self.api_client.api_client.nym_api.get_signer_status().await {
            Ok(response) => SigningStatus::Reachable { response },
            Err(err) => {
                warn!(
                    "{}: failed to retrieve signer chain status: {err}",
                    self.api_client
                );
                SigningStatus::Unreachable
            }
        }
    }
}

pub(crate) async fn check_client(raw_share: ContractVKShare) -> SignerResult {
    let signer_information: SignerInformation = (&raw_share).into();

    // 4. attempt to construct client instances out of them
    // (don't use `all_ecash_api_clients` as we want to treat each error individually;
    // for example during epoch advancement we still want to be able to perform monitoring
    // even if some shares are unverified)
    let Ok(client) = EcashApiClient::try_from(raw_share) else {
        return SignerStatus::ProvidedInvalidDetails.with_signer_information(signer_information);
    };

    let mut client = ClientUnderTest::new(client);

    // 5. check basic connection status - can you retrieve build information?
    if !client.try_retrieve_build_information().await {
        return SignerStatus::Unreachable.with_signer_information(signer_information);
    }

    // 6. check perceived chain status
    let local_chain_status = client.check_local_chain().await;

    // 7. check signer status
    let signing_status = client.check_signing_status().await;

    SignerStatus::Tested {
        result: SignerTestResult {
            reported_version: client.version().map(|v| v.to_string()).unwrap_or_default(),
            signing_status,
            local_chain_status,
        },
    }
    .with_signer_information(signer_information)
}
