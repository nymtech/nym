// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::handlers::ContractVersionSchemaResponse;
use nym_api_requests::models;
use utoipa::OpenApi;
use utoipauto::utoipauto;

// TODO once https://github.com/ProbablyClem/utoipauto/pull/38 is released:
// include ",./nym-api/nym-api-requests/src from nym-api-requests" (and other packages mentioned below)
// for automatic model discovery based on ToSchema / IntoParams implementation.
// Then you can remove `components(schemas)` manual imports below

#[utoipauto(paths = "./nym-api/src")]
#[derive(OpenApi)]
#[openapi(
    info(title = "Nym API"),
    tags(),
    components(schemas(
        models::CirculatingSupplyResponse,
        models::CoinSchema,
        nym_mixnet_contract_common::Interval,
        nym_api_requests::models::GatewayStatusReportResponse,
        nym_api_requests::models::GatewayUptimeHistoryResponse,
        nym_api_requests::models::HistoricalUptimeResponse,
        nym_api_requests::models::GatewayCoreStatusResponse,
        nym_api_requests::models::GatewayUptimeResponse,
        nym_api_requests::models::RewardEstimationResponse,
        nym_api_requests::models::UptimeResponse,
        nym_api_requests::models::ComputeRewardEstParam,
        nym_api_requests::models::MixNodeBondAnnotated,
        nym_api_requests::models::GatewayBondAnnotated,
        nym_api_requests::models::MixnodeTestResultResponse,
        nym_api_requests::models::StakeSaturationResponse,
        nym_api_requests::models::InclusionProbabilityResponse,
        nym_api_requests::models::AllInclusionProbabilitiesResponse,
        nym_api_requests::models::InclusionProbability,
        nym_api_requests::models::SelectionChance,
        crate::network::models::NetworkDetails,
        nym_config::defaults::NymNetworkDetails,
        nym_config::defaults::ChainDetails,
        nym_config::defaults::DenomDetailsOwned,
        nym_config::defaults::ValidatorDetails,
        nym_config::defaults::NymContracts,
        ContractVersionSchemaResponse,
        crate::network::models::ContractInformation<ContractVersionSchemaResponse>,
        nym_api_requests::models::ApiHealthResponse,
        nym_api_requests::models::ApiStatus,
        nym_bin_common::build_information::BinaryBuildInformationOwned,
        nym_api_requests::models::SignerInformationResponse,
        nym_api_requests::models::DescribedGateway,
        nym_api_requests::models::MixNodeDetailsSchema,
        nym_mixnet_contract_common::Gateway,
        nym_mixnet_contract_common::GatewayBond,
        nym_api_requests::models::NymNodeDescription,
        nym_api_requests::models::HostInformation,
        nym_api_requests::models::HostKeys,
        nym_node_requests::api::v1::node::models::AuxiliaryDetails,
        nym_api_requests::models::NetworkRequesterDetails,
        nym_api_requests::models::IpPacketRouterDetails,
        nym_api_requests::models::AuthenticatorDetails,
        nym_api_requests::models::WebSockets,
        nym_api_requests::nym_nodes::NodeRole,
        nym_api_requests::models::DescribedMixNode,
        nym_api_requests::ecash::VerificationKeyResponse,
        nym_api_requests::ecash::models::AggregatedExpirationDateSignatureResponse,
        nym_api_requests::ecash::models::AggregatedCoinIndicesSignatureResponse,
        nym_api_requests::ecash::models::EpochCredentialsResponse,
        nym_api_requests::ecash::models::IssuedCredentialResponse,
        nym_api_requests::ecash::models::IssuedTicketbookBody,
        nym_api_requests::ecash::models::BlindedSignatureResponse,
        nym_api_requests::ecash::models::PartialExpirationDateSignatureResponse,
        nym_api_requests::ecash::models::PartialCoinIndicesSignatureResponse,
        nym_api_requests::ecash::models::EcashTicketVerificationResponse,
        nym_api_requests::ecash::models::EcashTicketVerificationRejection,
        nym_api_requests::ecash::models::EcashBatchTicketRedemptionResponse,
        nym_api_requests::ecash::models::SpentCredentialsResponse,
        nym_api_requests::ecash::models::IssuedCredentialsResponse,
        nym_api_requests::nym_nodes::SkimmedNode,
        nym_api_requests::nym_nodes::BasicEntryInformation,
        nym_api_requests::nym_nodes::SemiSkimmedNode,
        nym_api_requests::nym_nodes::NodeRoleQueryParam,
    ))
)]
pub(super) struct ApiDoc;
