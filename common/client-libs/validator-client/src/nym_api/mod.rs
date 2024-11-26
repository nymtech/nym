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
use nym_api_requests::ecash::VerificationKeyResponse;
use nym_api_requests::models::{
    AnnotationResponse, ApiHealthResponse, LegacyDescribedMixNode, NodePerformanceResponse,
    NodeRefreshBody, NymNodeDescription,
};
use nym_api_requests::nym_nodes::PaginatedCachedNodesResponse;
use nym_api_requests::pagination::PaginatedResponse;
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
        ComputeRewardEstParam, GatewayBondAnnotated, GatewayCoreStatusResponse,
        GatewayStatusReportResponse, GatewayUptimeHistoryResponse, InclusionProbabilityResponse,
        LegacyDescribedGateway, MixNodeBondAnnotated, MixnodeCoreStatusResponse,
        MixnodeStatusReportResponse, MixnodeStatusResponse, MixnodeUptimeHistoryResponse,
        RewardEstimationResponse, StakeSaturationResponse, UptimeResponse,
    },
    nym_nodes::{CachedNodesResponse, SkimmedNode},
};
pub use nym_coconut_dkg_common::types::EpochId;
use nym_contracts_common::IdentityKey;
pub use nym_http_api_client::Client;
use nym_http_api_client::{ApiClient, NO_PARAMS};
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::{GatewayBond, IdentityKeyRef, NodeId, NymNodeDetails};
use time::format_description::BorrowedFormatItem;
use time::Date;
use tracing::instrument;

pub mod error;
pub mod routes;

pub fn rfc_3339_date() -> Vec<BorrowedFormatItem<'static>> {
    time::format_description::parse("[year]-[month]-[day]").unwrap()
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NymApiClientExt: ApiClient {
    async fn health(&self) -> Result<ApiHealthResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                routes::API_STATUS_ROUTES,
                routes::HEALTH,
            ],
            NO_PARAMS,
        )
        .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.get_json(&[routes::API_VERSION, routes::MIXNODES], NO_PARAMS)
            .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_gateways(&self) -> Result<Vec<GatewayBond>, NymAPIError> {
        self.get_json(&[routes::API_VERSION, routes::GATEWAYS], NO_PARAMS)
            .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_gateways_described(&self) -> Result<Vec<LegacyDescribedGateway>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::GATEWAYS, routes::DESCRIBED],
            NO_PARAMS,
        )
        .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnodes_described(&self) -> Result<Vec<LegacyDescribedMixNode>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::MIXNODES, routes::DESCRIBED],
            NO_PARAMS,
        )
        .await
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn get_nodes_described(
        &self,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<PaginatedResponse<NymNodeDescription>, NymAPIError> {
        let mut params = Vec::new();

        if let Some(page) = page {
            params.push(("page", page.to_string()))
        }

        if let Some(per_page) = per_page {
            params.push(("per_page", per_page.to_string()))
        }

        self.get_json(&[routes::API_VERSION, "nym-nodes", "described"], &params)
            .await
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn get_nym_nodes(
        &self,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<PaginatedResponse<NymNodeDetails>, NymAPIError> {
        let mut params = Vec::new();

        if let Some(page) = page {
            params.push(("page", page.to_string()))
        }

        if let Some(per_page) = per_page {
            params.push(("per_page", per_page.to_string()))
        }

        self.get_json(&[routes::API_VERSION, "nym-nodes", "bonded"], &params)
            .await
    }

    #[deprecated]
    #[tracing::instrument(level = "debug", skip_all)]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    /// retrieve basic information for nodes are capable of operating as an entry gateway
    /// this includes legacy gateways and nym-nodes
    #[instrument(level = "debug", skip(self))]
    async fn get_basic_entry_assigned_nodes(
        &self,
        semver_compatibility: Option<String>,
        no_legacy: bool,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<PaginatedCachedNodesResponse<SkimmedNode>, NymAPIError> {
        let mut params = Vec::new();

        if let Some(arg) = &semver_compatibility {
            params.push(("semver_compatibility", arg.clone()))
        }

        if no_legacy {
            params.push(("no_legacy", "true".to_string()))
        }

        if let Some(page) = page {
            params.push(("page", page.to_string()))
        }

        if let Some(per_page) = per_page {
            params.push(("per_page", per_page.to_string()))
        }

        self.get_json(
            &[
                routes::API_VERSION,
                "unstable",
                "nym-nodes",
                "skimmed",
                "entry-gateways",
                "all",
            ],
            &params,
        )
        .await
    }

    /// retrieve basic information for nodes that got assigned 'mixing' node in this epoch
    /// this includes legacy mixnodes and nym-nodes
    #[instrument(level = "debug", skip(self))]
    async fn get_basic_active_mixing_assigned_nodes(
        &self,
        semver_compatibility: Option<String>,
        no_legacy: bool,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<PaginatedCachedNodesResponse<SkimmedNode>, NymAPIError> {
        let mut params = Vec::new();

        if let Some(arg) = &semver_compatibility {
            params.push(("semver_compatibility", arg.clone()))
        }

        if no_legacy {
            params.push(("no_legacy", "true".to_string()))
        }

        if let Some(page) = page {
            params.push(("page", page.to_string()))
        }

        if let Some(per_page) = per_page {
            params.push(("per_page", per_page.to_string()))
        }

        self.get_json(
            &[
                routes::API_VERSION,
                "unstable",
                "nym-nodes",
                "skimmed",
                "mixnodes",
                "active",
            ],
            &params,
        )
        .await
    }

    /// retrieve basic information for nodes that got assigned 'mixing' node in this epoch
    /// this includes legacy mixnodes and nym-nodes
    #[instrument(level = "debug", skip(self))]
    async fn get_basic_mixing_capable_nodes(
        &self,
        semver_compatibility: Option<String>,
        no_legacy: bool,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<PaginatedCachedNodesResponse<SkimmedNode>, NymAPIError> {
        let mut params = Vec::new();

        if let Some(arg) = &semver_compatibility {
            params.push(("semver_compatibility", arg.clone()))
        }

        if no_legacy {
            params.push(("no_legacy", "true".to_string()))
        }

        if let Some(page) = page {
            params.push(("page", page.to_string()))
        }

        if let Some(per_page) = per_page {
            params.push(("per_page", per_page.to_string()))
        }

        self.get_json(
            &[
                routes::API_VERSION,
                "unstable",
                "nym-nodes",
                "skimmed",
                "mixnodes",
                "all",
            ],
            &params,
        )
        .await
    }
    #[instrument(level = "debug", skip(self))]

    async fn get_basic_nodes(
        &self,
        semver_compatibility: Option<String>,
        no_legacy: bool,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<PaginatedCachedNodesResponse<SkimmedNode>, NymAPIError> {
        let mut params = Vec::new();

        if let Some(arg) = &semver_compatibility {
            params.push(("semver_compatibility", arg.clone()))
        }

        if no_legacy {
            params.push(("no_legacy", "true".to_string()))
        }

        if let Some(page) = page {
            params.push(("page", page.to_string()))
        }

        if let Some(per_page) = per_page {
            params.push(("per_page", per_page.to_string()))
        }

        self.get_json(
            &[routes::API_VERSION, "unstable", "nym-nodes", "skimmed"],
            &params,
        )
        .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_active_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::MIXNODES, routes::ACTIVE],
            NO_PARAMS,
        )
        .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_rewarded_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::MIXNODES, routes::REWARDED],
            NO_PARAMS,
        )
        .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_report(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_history(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_core_status_count(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_status(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_reward_estimation(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn compute_mixnode_reward_estimation(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_stake_saturation(
        &self,
        mix_id: NodeId,
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnode_inclusion_probability(
        &self,
        mix_id: NodeId,
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

    #[instrument(level = "debug", skip(self))]
    async fn get_current_node_performance(
        &self,
        node_id: NodeId,
    ) -> Result<NodePerformanceResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                "nym-nodes",
                "performance",
                &node_id.to_string(),
            ],
            NO_PARAMS,
        )
        .await
    }

    async fn get_node_annotation(
        &self,
        node_id: NodeId,
    ) -> Result<AnnotationResponse, NymAPIError> {
        self.get_json(
            &[
                routes::API_VERSION,
                "nym-nodes",
                "annotation",
                &node_id.to_string(),
            ],
            NO_PARAMS,
        )
        .await
    }

    #[deprecated]
    async fn get_mixnode_avg_uptime(&self, mix_id: NodeId) -> Result<UptimeResponse, NymAPIError> {
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_mixnodes_blacklisted(&self) -> Result<Vec<NodeId>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::MIXNODES, routes::BLACKLISTED],
            NO_PARAMS,
        )
        .await
    }

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
    async fn get_gateways_blacklisted(&self) -> Result<Vec<IdentityKey>, NymAPIError> {
        self.get_json(
            &[routes::API_VERSION, routes::GATEWAYS, routes::BLACKLISTED],
            NO_PARAMS,
        )
        .await
    }

    #[instrument(level = "debug", skip(self, request_body))]
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

    #[instrument(level = "debug", skip(self, request_body))]
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

    #[instrument(level = "debug", skip(self, request_body))]
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

    #[deprecated]
    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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

    async fn force_refresh_describe_cache(
        &self,
        request: &NodeRefreshBody,
    ) -> Result<(), NymAPIError> {
        self.post_json(
            &[routes::API_VERSION, "nym-nodes", "refresh-described"],
            NO_PARAMS,
            request,
        )
        .await
    }

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
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
