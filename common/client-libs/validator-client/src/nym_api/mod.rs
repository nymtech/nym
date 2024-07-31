// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nym_api::error::NymAPIError;
use crate::nym_api::routes::{ecash, CORE_STATUS_COUNT, SINCE_ARG};
use async_trait::async_trait;
use nym_api_requests::ecash::models::{
    AggregatedCoinIndicesSignatureResponse, AggregatedExpirationDateSignatureResponse,
    BatchRedeemTicketsBody, EcashBatchTicketRedemptionResponse, EcashTicketVerificationResponse,
    VerifyEcashTicketBody,
};
use nym_api_requests::nym_nodes::{CachedNodesResponse, SkimmedNode};
use nym_http_api_client::{ApiClient, NO_PARAMS};
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::{GatewayBond, IdentityKeyRef, MixId};
use time::format_description::BorrowedFormatItem;
use time::Date;

pub mod error;
pub mod routes;

use nym_api_requests::ecash::VerificationKeyResponse;
pub use nym_api_requests::{
    ecash::{
        models::{
            EpochCredentialsResponse, IssuedCredentialResponse, IssuedCredentialsResponse,
            IssuedTicketbook, IssuedTicketbookBody, SpentCredentialsResponse,
        },
        BlindSignRequestBody, BlindedSignatureResponse, CredentialsRequestBody,
        PartialCoinIndicesSignatureResponse, PartialExpirationDateSignatureResponse,
        VerifyEcashCredentialBody,
    },
    models::{
        ComputeRewardEstParam, DescribedGateway, GatewayBondAnnotated, GatewayCoreStatusResponse,
        GatewayStatusReportResponse, GatewayUptimeHistoryResponse, InclusionProbabilityResponse,
        MixNodeBondAnnotated, MixnodeCoreStatusResponse, MixnodeStatusReportResponse,
        MixnodeStatusResponse, MixnodeUptimeHistoryResponse, RewardEstimationResponse,
        StakeSaturationResponse, UptimeResponse,
    },
};
pub use nym_coconut_dkg_common::types::EpochId;
pub use nym_http_api_client::Client;

pub fn rfc_3339_date() -> Vec<BorrowedFormatItem<'static>> {
    time::format_description::parse("[year]-[month]-[day]").unwrap()
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NymApiClientExt: ApiClient {
    async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.get_json(&[routes::API_VERSION, routes::MIXNODES], NO_PARAMS)
            .await
    }

    async fn get_mixnodes_detailed(&self) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_gateways_detailed(&self) -> Result<Vec<GatewayBondAnnotated>, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::GATEWAYS,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_mixnodes_detailed_unfiltered(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::DETAILED_UNFILTERED,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_gateways(&self) -> Result<Vec<GatewayBond>, NymAPIError> {
        self.get_json(&[routes::API_VERSION, routes::GATEWAYS], NO_PARAMS)
            .await
    }

    async fn get_gateways_described(&self) -> Result<Vec<DescribedGateway>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::GATEWAYS, routes::DESCRIBED],
            NO_PARAMS,
        )
        .await
    }

    async fn get_basic_mixnodes(
        &self,
        semver_compatibility: Option<String>,
    ) -> Result<CachedNodesResponse<SkimmedNode>, NymAPIError> {
        let params = if let Some(semver_compatibility) = &semver_compatibility {
            vec![("semver_compatibility", semver_compatibility.as_str())]
        } else {
            vec![]
        };

        self.get_json(
            &[
                routes::API_VERSION,
                "unstable",
                "nym-nodes",
                "mixnodes",
                "skimmed",
            ],
            &params,
        )
        .await
    }

    async fn get_basic_gateways(
        &self,
        semver_compatibility: Option<String>,
    ) -> Result<CachedNodesResponse<SkimmedNode>, NymAPIError> {
        let params = if let Some(semver_compatibility) = &semver_compatibility {
            vec![("semver_compatibility", semver_compatibility.as_str())]
        } else {
            vec![]
        };

        self.get_json(
            &[
                routes::API_VERSION,
                "unstable",
                "nym-nodes",
                "gateways",
                "skimmed",
            ],
            &params,
        )
        .await
    }

    async fn get_active_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::MIXNODES, routes::ACTIVE],
            NO_PARAMS,
        )
        .await
    }

    async fn get_active_mixnodes_detailed(&self) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::ACTIVE,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_rewarded_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::MIXNODES, routes::REWARDED],
            NO_PARAMS,
        )
        .await
    }

    async fn get_mixnode_report(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeStatusReportResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::REPORT,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_gateway_report(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<GatewayStatusReportResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::GATEWAY,
                identity,
                routes::REPORT,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_mixnode_history(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeUptimeHistoryResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::HISTORY,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_gateway_history(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<GatewayUptimeHistoryResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::GATEWAY,
                identity,
                routes::HISTORY,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_rewarded_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::REWARDED,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<GatewayCoreStatusResponse, NymAPIError> {
        if let Some(since) = since {
            self.get_json(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::GATEWAY,
                    identity,
                    CORE_STATUS_COUNT,
                ],
                &[(SINCE_ARG, since.to_string())],
            )
            .await
        } else {
            self.get_json(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::GATEWAY,
                    identity,
                ],
                NO_PARAMS,
            )
            .await
        }
    }

    async fn get_mixnode_core_status_count(
        &self,
        mix_id: MixId,
        since: Option<i64>,
    ) -> Result<MixnodeCoreStatusResponse, NymAPIError> {
        if let Some(since) = since {
            self.get_json(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::MIXNODE,
                    &mix_id.to_string(),
                    CORE_STATUS_COUNT,
                ],
                &[(SINCE_ARG, since.to_string())],
            )
            .await
        } else {
            self.get_json(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::MIXNODE,
                    &mix_id.to_string(),
                    CORE_STATUS_COUNT,
                ],
                NO_PARAMS,
            )
            .await
        }
    }

    async fn get_mixnode_status(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeStatusResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::STATUS,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_mixnode_reward_estimation(
        &self,
        mix_id: MixId,
    ) -> Result<RewardEstimationResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::REWARD_ESTIMATION,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn compute_mixnode_reward_estimation(
        &self,
        mix_id: MixId,
        request_body: &ComputeRewardEstParam,
    ) -> Result<RewardEstimationResponse, NymAPIError> {
        self.post_json(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::COMPUTE_REWARD_ESTIMATION,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    async fn get_mixnode_stake_saturation(
        &self,
        mix_id: MixId,
    ) -> Result<StakeSaturationResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::STAKE_SATURATION,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_mixnode_inclusion_probability(
        &self,
        mix_id: MixId,
    ) -> Result<InclusionProbabilityResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::INCLUSION_CHANCE,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_mixnode_avg_uptime(&self, mix_id: MixId) -> Result<UptimeResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::AVG_UPTIME,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, NymAPIError> {
        self.post_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::ECASH_BLIND_SIGN,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    async fn verify_ecash_ticket(
        &self,
        request_body: &VerifyEcashTicketBody,
    ) -> Result<EcashTicketVerificationResponse, NymAPIError> {
        self.post_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::VERIFY_ECASH_TICKET,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    async fn batch_redeem_ecash_tickets(
        &self,
        request_body: &BatchRedeemTicketsBody,
    ) -> Result<EcashBatchTicketRedemptionResponse, NymAPIError> {
        self.post_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::BATCH_REDEEM_ECASH_TICKETS,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    async fn double_spending_filter_v1(&self) -> Result<SpentCredentialsResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::DOUBLE_SPENDING_FILTER_V1,
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn partial_expiration_date_signatures(
        &self,
        expiration_date: Option<Date>,
    ) -> Result<PartialExpirationDateSignatureResponse, NymAPIError> {
        let params = match expiration_date {
            None => Vec::new(),
            Some(exp) => vec![(
                ecash::EXPIRATION_DATE_PARAM,
                exp.format(&rfc_3339_date()).unwrap(),
            )],
        };

        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::PARTIAL_EXPIRATION_DATE_SIGNATURES,
            ],
            &params,
        )
        .await
    }

    async fn partial_coin_indices_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<PartialCoinIndicesSignatureResponse, NymAPIError> {
        let params = match epoch_id {
            None => Vec::new(),
            Some(epoch_id) => vec![(ecash::EPOCH_ID_PARAM, epoch_id.to_string())],
        };

        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::PARTIAL_COIN_INDICES_SIGNATURES,
            ],
            &params,
        )
        .await
    }

    async fn global_expiration_date_signatures(
        &self,
        expiration_date: Option<Date>,
    ) -> Result<AggregatedExpirationDateSignatureResponse, NymAPIError> {
        let params = match expiration_date {
            None => Vec::new(),
            Some(exp) => vec![(
                ecash::EXPIRATION_DATE_PARAM,
                exp.format(&rfc_3339_date()).unwrap(),
            )],
        };

        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::GLOBAL_EXPIRATION_DATE_SIGNATURES,
            ],
            &params,
        )
        .await
    }

    async fn global_coin_indices_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<AggregatedCoinIndicesSignatureResponse, NymAPIError> {
        let params = match epoch_id {
            None => Vec::new(),
            Some(epoch_id) => vec![(ecash::EPOCH_ID_PARAM, epoch_id.to_string())],
        };

        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::GLOBAL_COIN_INDICES_SIGNATURES,
            ],
            &params,
        )
        .await
    }

    async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<VerificationKeyResponse, NymAPIError> {
        let params = match epoch_id {
            None => Vec::new(),
            Some(epoch_id) => vec![(ecash::EPOCH_ID_PARAM, epoch_id.to_string())],
        };
        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::ecash::MASTER_VERIFICATION_KEY,
            ],
            &params,
        )
        .await
    }

    async fn epoch_credentials(
        &self,
        dkg_epoch: EpochId,
    ) -> Result<EpochCredentialsResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::ECASH_EPOCH_CREDENTIALS,
                &dkg_epoch.to_string(),
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<IssuedCredentialResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::ECASH_ISSUED_CREDENTIAL,
                &credential_id.to_string(),
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<IssuedCredentialsResponse, NymAPIError> {
        self.post_json(
            &[
                routes::API_VERSION,
                routes::ECASH_ROUTES,
                routes::ECASH_ISSUED_CREDENTIALS,
            ],
            NO_PARAMS,
            &CredentialsRequestBody {
                credential_ids,
                pagination: None,
            },
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NymApiClientExt for Client {}
