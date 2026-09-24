// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::{ContractInformation, NetworkDetails};
use crate::node_status_api::models::AxumResult;
use crate::signers_cache::handlers::signers_routes;
use crate::support::config::CHAIN_STALL_THRESHOLD;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Router;
use nym_api_requests::models::{
    ChainBlocksStatusResponse, ChainBlocksStatusResponseBody, ChainStatus, ChainStatusResponse,
};
use nym_api_requests::signable::SignableMessageBody;
use nym_contracts_common::ContractBuildInformation;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_network_defaults::mainnet;
use std::collections::HashMap;
use time::OffsetDateTime;
use tower_http::compression::CompressionLayer;
use utoipa::ToSchema;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/details", axum::routing::get(network_details))
        .route("/chain-status", axum::routing::get(chain_status))
        .route(
            "/chain-blocks-status",
            axum::routing::get(chain_blocks_status),
        )
        .route("/nym-contracts", axum::routing::get(nym_contracts))
        .route(
            "/nym-contracts-detailed",
            axum::routing::get(nym_contracts_detailed),
        )
        .nest("/signers", signers_routes())
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network",
    path = "/details",
    responses(
        (status = 200, content(
            (NetworkDetails = "application/json"),
            (NetworkDetails = "application/yaml"),
            (NetworkDetails = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn network_details(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<NetworkDetails> {
    let output = output.output.unwrap_or_default();

    let mut details: NetworkDetails = state.network_details().to_owned().into();

    // clients using the v1 endpoint don't support dynamic dns fallbacks, so on mainnet we
    // always serve them the fixed v1 api urls rather than whatever the current config holds
    if details.network.network_name == mainnet::NETWORK_NAME {
        details.network = details.network.with_pinned_api_urls();
    }

    output.to_response(details)
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network",
    path = "/chain-status",
    responses(
        (status = 200, content(
            (ChainStatusResponse = "application/json"),
            (ChainStatusResponse = "application/yaml"),
            (ChainStatusResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn chain_status(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<ChainStatusResponse>> {
    let output = output.output.unwrap_or_default();

    let chain_status = state
        .chain_status_cache
        .get_or_refresh(&state.nyxd_client)
        .await?;

    let connected_nyxd = state.network_details.connected_nyxd;

    Ok(output.to_response(ChainStatusResponse {
        connected_nyxd,
        status: chain_status,
    }))
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network",
    path = "/chain-blocks-status",
    responses(
        (status = 200, content(
            (ChainBlocksStatusResponse = "application/json"),
            (ChainBlocksStatusResponse = "application/yaml"),
            (ChainBlocksStatusResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn chain_blocks_status(
    Query(params): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<ChainBlocksStatusResponse> {
    let output = params.get_output();

    let current_time = OffsetDateTime::now_utc();
    let latest_cached_block = state
        .chain_status_cache
        .get_or_refresh(&state.nyxd_client)
        .await
        .ok();
    let chain_status = latest_cached_block
        .as_ref()
        .map(|detailed| detailed.stall_status(current_time, CHAIN_STALL_THRESHOLD))
        .unwrap_or(ChainStatus::Unknown);

    output.to_response(
        ChainBlocksStatusResponseBody {
            current_time,
            latest_cached_block,
            chain_status,
        }
        .sign(state.private_signing_key()),
    )
}

// it's used for schema generation so dead_code is fine
#[allow(dead_code)]
#[derive(ToSchema)]
#[schema(title = "ContractVersion")]
pub(crate) struct ContractVersionSchemaResponse {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to its implementation somehow)
    pub version: String,
}

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
pub struct ContractInformationContractVersion {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<ContractVersionSchemaResponse>,
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network",
    path = "/nym-contracts",
    responses(
        (status = 200, content(
            (HashMap<String, ContractInformationContractVersion> = "application/json"),
            (HashMap<String, ContractInformationContractVersion> = "application/yaml"),
            (HashMap<String, ContractInformationContractVersion> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn nym_contracts(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<HashMap<String, ContractInformation<cw2::ContractVersion>>>> {
    let output = output.output.unwrap_or_default();

    let contract_info = state
        .contract_info_cache
        .get_or_refresh(&state.nyxd_client)
        .await?;

    Ok(output.to_response(
        contract_info
            .iter()
            .map(|(contract, info)| {
                (
                    contract.to_owned(),
                    ContractInformation {
                        address: info.address.as_ref().map(|a| a.to_string()),
                        details: info.base.clone(),
                    },
                )
            })
            .collect::<HashMap<_, _>>(),
    ))
}

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
pub struct ContractInformationBuildInformation {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<ContractBuildInformation>,
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network",
    path = "/nym-contracts-detailed",
    responses(
        (status = 200, content(
            (HashMap<String, ContractInformationBuildInformation> = "application/json"),
            (HashMap<String, ContractInformationBuildInformation> = "application/yaml"),
            (HashMap<String, ContractInformationBuildInformation> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn nym_contracts_detailed(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<HashMap<String, ContractInformation<ContractBuildInformation>>>> {
    let output = output.output.unwrap_or_default();

    let contract_info = state
        .contract_info_cache
        .get_or_refresh(&state.nyxd_client)
        .await?;

    Ok(output.to_response(
        contract_info
            .iter()
            .map(|(contract, info)| {
                (
                    contract.to_owned(),
                    ContractInformation {
                        address: info.address.as_ref().map(|a| a.to_string()),
                        details: info.detailed.clone(),
                    },
                )
            })
            .collect::<HashMap<_, _>>(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecash::tests::build_dummy_ecash_state;
    use crate::network::models::NetworkDetailsV2;
    use crate::support::caching::cache::SharedCache;
    use crate::support::config;
    use crate::support::http::state::test_helpers::build_app_state;
    use crate::support::storage::NymApiStorage;
    use axum_test::http::StatusCode;
    use axum_test::TestServer;
    use nym_network_defaults::{v1, v2, ApiUrl};

    async fn test_server(network: v2::NymNetworkDetails) -> TestServer {
        let storage = NymApiStorage::init_in_memory().await.unwrap();

        let mut cfg = config::Config::new("test");
        cfg.ecash_signer.enabled = false;
        let bundle = build_dummy_ecash_state(&cfg, storage.clone(), [7u8; 32]).await;

        let mut app_state = build_app_state(
            storage,
            bundle.ecash_state,
            bundle.real_client,
            SharedCache::new(),
        );
        app_state.network_details = NetworkDetailsV2::new("localhost".to_string(), network);

        TestServer::new(
            Router::new()
                .nest("/v1/network", routes())
                .with_state(app_state),
        )
    }

    fn to_api_urls(urls: &[nym_network_defaults::ApiUrlConst]) -> Vec<ApiUrl> {
        urls.iter().copied().map(Into::into).collect()
    }

    async fn get_details(server: &TestServer) -> NetworkDetails {
        let res = server.get("/v1/network/details").await;
        assert_eq!(res.status_code(), StatusCode::OK);
        res.json()
    }

    // As of nymtech/nym-vpn-client#6279 (to be released in vpn-client v2026.13) clients should
    // depend on v2/network/details which includes fallback dns information for any API urls as part
    // of the config. PREVIOUS to this change, the vpn-client relied on hard coded addresses for DNS
    // fallbacks in environments where nameservers for internal lookups were unreliable or blocked.
    // This meant that changes to the set of API urls could break connections for clients in
    // censoring regions. For older clients (that still use v1/network/details this will continue to
    // be the case - so those URLs need to remain unchanged.
    #[tokio::test]
    async fn network_details_returns_v1_api_urls() {
        let server = test_server(v2::NymNetworkDetails::new_mainnet()).await;
        let network = get_details(&server).await.network;

        assert_eq!(network.nym_api_urls, Some(to_api_urls(v1::NYM_APIS)));
        assert_eq!(
            network.nym_vpn_api_urls,
            Some(to_api_urls(v1::NYM_VPN_APIS))
        );
        assert_eq!(network.nym_vpn_api_url.as_deref(), Some(v1::NYM_VPN_API));
    }

    #[tokio::test]
    async fn network_details_pins_v1_api_urls_on_mainnet_regardless_of_config() {
        let mut network = v2::NymNetworkDetails::new_mainnet();
        network.set_nym_api_urls(vec![ApiUrl {
            url: "https://not-a-v1-url.example.com/api/".to_string(),
            front_hosts: None,
        }]);
        network.set_nym_vpn_api_urls(vec![ApiUrl {
            url: "https://not-a-v1-vpn-url.example.com/api/".to_string(),
            front_hosts: None,
        }]);

        let server = test_server(network).await;
        let network = get_details(&server).await.network;

        assert_eq!(network.nym_api_urls, Some(to_api_urls(v1::NYM_APIS)));
        assert_eq!(
            network.nym_vpn_api_urls,
            Some(to_api_urls(v1::NYM_VPN_APIS))
        );
        assert_eq!(network.nym_vpn_api_url.as_deref(), Some(v1::NYM_VPN_API));
    }

    #[tokio::test]
    async fn network_details_does_not_pin_api_urls_on_other_networks() {
        let sandbox = v2::NymNetworkDetails::new_sandbox();
        let expected: v1::NymNetworkDetails = sandbox.clone().into();

        let server = test_server(sandbox).await;
        let network = get_details(&server).await.network;

        assert_eq!(network.nym_api_urls, expected.nym_api_urls);
        assert_eq!(network.nym_vpn_api_urls, expected.nym_vpn_api_urls);
        assert_eq!(network.nym_vpn_api_url, expected.nym_vpn_api_url);
    }
}
