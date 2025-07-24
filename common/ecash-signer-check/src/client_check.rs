// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{LocalChainStatus, SigningStatus, TypedSignerResult};
use nym_ecash_signer_check_types::dealer_information::RawDealerInformation;
use nym_ecash_signer_check_types::status::{SignerStatus, SignerTestResult};
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::BinaryBuildInformationOwned;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::{
    ContractVKShare, DealerDetails,
};
use nym_validator_client::NymApiClient;
use std::time::Duration;
use tracing::{error, warn};
use url::Url;

pub(crate) mod chain_status {

    // Dorina
    pub(crate) const MINIMUM_VERSION_LEGACY: semver::Version = semver::Version::new(1, 1, 51);

    // Gruyere
    pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 64);
}

pub(crate) mod signing_status {
    // Magura (possibly earlier)
    pub(crate) const MINIMUM_LEGACY_VERSION: semver::Version = semver::Version::new(1, 1, 46);

    // Gruyere
    pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 64);
}

struct ClientUnderTest {
    api_client: NymApiClient,
    build_info: Option<BinaryBuildInformationOwned>,
}

impl ClientUnderTest {
    pub(crate) fn new(api_url: &Url) -> Self {
        ClientUnderTest {
            api_client: NymApiClient::new(api_url.clone()),
            build_info: None,
        }
    }

    pub(crate) async fn try_retrieve_build_information(&mut self) -> bool {
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.api_client.nym_api.build_information(),
        )
        .await
        {
            Ok(Ok(build_information)) => {
                self.build_info = Some(build_information);
                true
            }
            Ok(Err(err)) => {
                warn!("{}: failed to retrieve build information: {err}. the signer is most likely down", self.api_client.api_url());
                false
            }
            Err(_timeout) => {
                warn!(
                    "{}: timed out while attempting to retrieve build information",
                    self.api_client.api_url()
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
                        self.api_client.api_url(),
                        build_info.build_version
                    )
                })
                .ok()
        })
    }

    pub(crate) fn supports_legacy_signing_status_query(&self) -> bool {
        let Some(version) = self.version() else {
            return false;
        };
        version >= signing_status::MINIMUM_LEGACY_VERSION
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

    pub(crate) fn supports_legacy_chain_status_query(&self) -> bool {
        let Some(version) = self.version() else {
            return false;
        };
        version >= chain_status::MINIMUM_VERSION_LEGACY
    }

    pub(crate) async fn check_local_chain(&self) -> LocalChainStatus {
        // check if it at least supports legacy query
        if !self.supports_legacy_chain_status_query() {
            return LocalChainStatus::Outdated;
        }

        // check if it supports the current query
        if self.supports_chain_status_query() {
            return match self.api_client.nym_api.get_chain_blocks_status().await {
                Ok(status) => LocalChainStatus::Reachable {
                    response: Box::new(status),
                },
                Err(err) => {
                    warn!(
                        "{}: failed to retrieve local chain status: {err}",
                        self.api_client.api_url()
                    );
                    LocalChainStatus::Unreachable
                }
            };
        }

        // fallback to the legacy query
        match self.api_client.nym_api.get_chain_status().await {
            Ok(status) => LocalChainStatus::ReachableLegacy {
                response: Box::new(status),
            },
            Err(err) => {
                warn!(
                    "{}: failed to retrieve [legacy] local chain status: {err}",
                    self.api_client.api_url()
                );
                LocalChainStatus::Unreachable
            }
        }
    }

    pub(crate) async fn check_signing_status(&self) -> SigningStatus {
        // check if it at least supports legacy query
        if !self.supports_legacy_signing_status_query() {
            return SigningStatus::Outdated;
        }

        // check if it supports the current query
        if self.supports_signing_status_query() {
            return match self.api_client.nym_api.get_signer_status().await {
                Ok(response) => SigningStatus::Reachable {
                    response: Box::new(response),
                },
                Err(err) => {
                    warn!(
                        "{}: failed to retrieve signer chain status: {err}",
                        self.api_client.api_url()
                    );
                    SigningStatus::Unreachable
                }
            };
        }

        // fallback to the legacy query
        match self.api_client.nym_api.get_signer_information().await {
            Ok(status) => SigningStatus::ReachableLegacy {
                response: Box::new(status),
            },
            Err(err) => {
                warn!(
                    "{}: failed to retrieve [legacy] signer chain status: {err}",
                    self.api_client.api_url()
                );
                // NOTE: this might equally mean the signing is disabled
                SigningStatus::Unreachable
            }
        }
    }
}

pub(crate) async fn check_client(
    dealer_details: DealerDetails,
    dkg_epoch: u64,
    contract_share: Option<&ContractVKShare>,
) -> TypedSignerResult {
    let dealer_information = RawDealerInformation::new(&dealer_details, contract_share);

    // 7. attempt to construct client instances out of them
    let Ok(parsed_information) = dealer_information.parse() else {
        return SignerStatus::ProvidedInvalidDetails.with_details(dealer_information, dkg_epoch);
    };

    let mut client = ClientUnderTest::new(&parsed_information.announce_address);

    // 8. check basic connection status - can you retrieve build information?
    if !client.try_retrieve_build_information().await {
        return SignerStatus::Unreachable.with_details(dealer_information, dkg_epoch);
    }

    // 9. check perceived chain status
    let local_chain_status = client.check_local_chain().await;

    // 10. check signer status
    let signing_status = client.check_signing_status().await;

    SignerStatus::Tested {
        result: SignerTestResult {
            reported_version: client.version().map(|v| v.to_string()).unwrap_or_default(),
            signing_status,
            local_chain_status,
        },
    }
    .with_details(dealer_information, dkg_epoch)
}
