// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![allow(deprecated)]
use crate::network::handlers::ContractVersionSchemaResponse;
use utoipa::OpenApi;
use utoipauto::utoipauto;

// TODO once https://github.com/ProbablyClem/utoipauto/pull/38 is released:
// include ",./nym-api/nym-api-requests/src from nym-api-requests" (and other packages mentioned below)
// for automatic model discovery based on ToSchema / IntoParams implementation.
// Then you can remove `components(schemas)` manual imports below

// dependencies which have derive(ToSchema) behind a feature flag with cfg_attr
// cannot be autodiscovered because proc macros run before feature flags.
// Tracking issue: https://github.com/ProbablyClem/utoipauto/issues/13

#[utoipauto(paths = "./nym-api/src,
    ./nym-api/nym-api-requests/src from nym-api-requests,
    ./common/nym_offline_compact_ecash/src from nym-compact-ecash,
    ./common/config/src from nym-config,
    ./common/ticketbooks-merkle/src from nym-ticketbooks-merkle,
    ./common/nym_offline_compact_ecash/src from nym_compact_ecash")]
#[derive(OpenApi)]
#[openapi(
    info(title = "Nym API"),
    servers(
        (url = "/api", description = "Main Nym Api Server"),
        (url = "/", description = "Auxiliary Nym Api Instances"),
        (url = "/", description = "Local Development Server")
    ),
    tags(),
    components(schemas(
        nym_mixnet_contract_common::Interval,
        nym_config::defaults::NymNetworkDetails,
        nym_config::defaults::ChainDetails,
        nym_config::defaults::DenomDetailsOwned,
        nym_config::defaults::ValidatorDetails,
        nym_config::defaults::NymContracts,
        ContractVersionSchemaResponse,
        nym_bin_common::build_information::BinaryBuildInformationOwned,
        nym_node_requests::api::v1::node::models::AuxiliaryDetails,
    ))
)]
pub(crate) struct ApiDoc;
