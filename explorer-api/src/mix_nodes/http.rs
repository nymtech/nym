use crate::mix_node::models::{MixnodeStatus, PrettyDetailedMixNodeBond};
use crate::mix_nodes::models::{MixNodeActiveSetSummary, MixNodeSummary};
use crate::state::ExplorerApiStateContext;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub fn mix_nodes_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: list,
        list_active_set,
        list_inactive_set,
        list_standby_set,
        summary
    ]
}

#[openapi(tag = "mix_nodes")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    Json(state.inner.mix_nodes.get_detailed_mixnodes().await)
}

#[openapi(tag = "mix_nodes")]
#[get("/active-set/active")]
pub(crate) async fn list_active_set(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    Json(get_mixnodes_by_status(
        state.inner.mix_nodes.get_detailed_mixnodes().await,
        MixnodeStatus::Active,
    ))
}

#[openapi(tag = "mix_nodes")]
#[get("/active-set/inactive")]
pub(crate) async fn list_inactive_set(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    Json(get_mixnodes_by_status(
        state.inner.mix_nodes.get_detailed_mixnodes().await,
        MixnodeStatus::Inactive,
    ))
}

#[openapi(tag = "mix_nodes")]
#[get("/active-set/standby")]
pub(crate) async fn list_standby_set(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    Json(get_mixnodes_by_status(
        state.inner.mix_nodes.get_detailed_mixnodes().await,
        MixnodeStatus::Standby,
    ))
}

#[openapi(tag = "mix_nodes")]
#[get("/summary")]
pub(crate) async fn summary(state: &State<ExplorerApiStateContext>) -> Json<MixNodeSummary> {
    Json(get_mixnode_summary(state).await)
}

pub(crate) async fn get_mixnode_summary(state: &State<ExplorerApiStateContext>) -> MixNodeSummary {
    let mixnodes = state.inner.mix_nodes.get_detailed_mixnodes().await;
    let active = get_mixnodes_by_status(mixnodes.clone(), MixnodeStatus::Active).len();
    let standby = get_mixnodes_by_status(mixnodes.clone(), MixnodeStatus::Standby).len();
    let inactive = get_mixnodes_by_status(mixnodes.clone(), MixnodeStatus::Inactive).len();
    MixNodeSummary {
        count: mixnodes.len(),
        activeset: MixNodeActiveSetSummary {
            active,
            standby,
            inactive,
        },
    }
}

fn get_mixnodes_by_status(
    all_mixnodes: Vec<PrettyDetailedMixNodeBond>,
    status: MixnodeStatus,
) -> Vec<PrettyDetailedMixNodeBond> {
    all_mixnodes
        .into_iter()
        .filter(|mixnode| mixnode.status == status)
        .collect()
}
