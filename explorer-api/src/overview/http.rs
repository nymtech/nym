use nym_validator_client::models::NymNodeData;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use crate::mix_nodes::http::get_mixnode_summary;
use crate::overview::models::{NymNodeSummary, OverviewSummary, RoleSummary};
use crate::state::ExplorerApiStateContext;

pub fn overview_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: summary]
}

fn get_nym_nodes_by_role(nodes: &Vec<NymNodeData>) -> RoleSummary {
    let mut summary = RoleSummary::default();

    for node in nodes {
        if node.declared_role.entry {
            summary.entry += 1;
        }
        if node.declared_role.exit_ipr {
            summary.exit_ipr += 1;
        }
        if node.declared_role.exit_nr {
            summary.exit_nr += 1;
        }
        if node.declared_role.mixnode {
            summary.mixnode += 1;
        }
    }

    summary
}

#[openapi(tag = "overview")]
#[get("/summary")]
pub(crate) async fn summary(state: &State<ExplorerApiStateContext>) -> Json<OverviewSummary> {
    let nym_nodes = state
        .inner
        .nymnodes
        .get_bonded_nymnodes_descriptions()
        .await;
    let roles = get_nym_nodes_by_role(&nym_nodes);

    Json(OverviewSummary {
        mixnodes: get_mixnode_summary(state).await,
        validators: state.inner.validators.get_validator_summary().await,
        gateways: state.inner.gateways.get_gateway_summary().await,
        nymnodes: NymNodeSummary {
            count: nym_nodes.len(),
            roles,
        },
    })
}
