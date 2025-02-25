#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use crate::{
    node_status_api::models::{AxumErrorResponse, AxumResult},
    support::http::{helpers::NodeIdParam, topology_cache::{TopologyCache, PayloadFormat}},
};

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use nym_mixnet_contract_common::{Interval, NodeId};
use nym_api_requests::models::RewardedSetResponse;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

type SkimmedNodes = AxumResult<Json<TopologyResponse>>;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct TopologyParams {
    #[allow(dead_code)]
    semver_compatibility: Option<String>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    epoch_id: Option<u32>,
}

#[allow(deprecated)]
pub(crate) fn topology_routes() -> Router<Arc<TopologyCache>> {
    Router::new()
        .route("layer-assignments", get(layer_assignments))
        .nest(
            "/skimmed",
            Router::new()
                .route("/", get(nodes_basic_all))
                .route("batch", post(nodes_basic_batch))
                .route("/:node_id", get(node_basic)),
        )
    // // NOT IMPLEMENTED
    // .nest(
    //     "/semi-skimmed",
    //     Router::new().route("/", get(nodes_expanded_all)),
    // )
    // .nest(
    //     "/full-fat",
    //     Router::new().route("/", get(nodes_detailed_all)),
    // )
}

async fn nodes_basic_all(
    State(state): State<Arc<TopologyCache>>,
    Query(query_params): Query<TopologyParams>,
) -> AxumResult<Json<TopologyResponse>> {
	Err(AxumErrorResponse::not_implemented())
}

async fn nodes_basic_batch(
    State(state): State<Arc<TopologyCache>>,
    Query(query_params): Query<TopologyParams>,
    Json(node_ids): Json<Vec<NodeId>>,
) -> AxumResult<Json<TopologyResponse>> {
	Err(AxumErrorResponse::not_implemented())
}

async fn node_basic(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<Arc<TopologyCache>>,
    Query(query_params): Query<TopologyParams>,
) -> AxumResult<Json<TopologyResponse>> {
	Err(AxumErrorResponse::not_implemented())
}

async fn layer_assignments(
    State(state): State<Arc<TopologyCache>>,
    Query(query_params): Query<TopologyParams>,
) -> AxumResult<Json<LayerAssignmentsResponse>> {
	Err(AxumErrorResponse::not_implemented())
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
struct LayerAssignmentsResponse {
    pub status: Option<TopologyRequestStatus>,

    pub assignments: RewardedSetResponse,

}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
struct TopologyResponse {
    pub status: Option<TopologyRequestStatus>,

    payload: Vec<u8>,
    payload_format: Option<PayloadFormat>,
    payload_signature: Option<Vec<u8>>,

    current_topology_hash: Option<Vec<u8>>,
    topology_signature: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TopologyRequestStatus {
    NoUpdates,
    Fresh(Interval),
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use axum_test::TestServer;
    use nym_topology::NymTopology;
    use time::OffsetDateTime;

    use crate::support::http::topology_cache::Epoch;

    use super::*;

	fn build_test_topology_cache() -> TopologyCache {

		let current_epoch = Epoch {
			id: 123,
			current_epoch_start: OffsetDateTime::now_utc(),
			epoch_length: Duration::from_secs(120),
		};

		let topology = NymTopology::default();

		TopologyCache::new(current_epoch, topology);
		todo!();
	}

    #[tokio::test]
    async fn test_topology_basic() -> Result<(), Box<dyn ::std::error::Error>> {
        let state = Arc::new(build_test_topology_cache());
        let app = topology_routes().with_state(state);

        let server = TestServer::new(app)?;

        let response = server
            .get(&"/layer_assignments")
            .await;

        response.assert_text("hello!");
		Ok(())
    }
}
