use crate::mix_node::models::{MixnodeStatus, PrettyDetailedMixNodeBond};
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
        list_standby_set
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
#[get("/active")]
pub(crate) async fn list_active_set(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    let mixnodes = state.inner.mix_nodes.get_detailed_mixnodes().await;
    let filtered = mixnodes
        .into_iter()
        .filter(|mixnode| mixnode.status == MixnodeStatus::Active)
        .collect();
    Json(filtered)
}

#[openapi(tag = "mix_nodes")]
#[get("/inactive")]
pub(crate) async fn list_inactive_set(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    let mixnodes = state.inner.mix_nodes.get_detailed_mixnodes().await;
    let filtered = mixnodes
        .into_iter()
        .filter(|mixnode| mixnode.status == MixnodeStatus::Inactive)
        .collect();
    Json(filtered)
}

#[openapi(tag = "mix_nodes")]
#[get("/standby")]
pub(crate) async fn list_standby_set(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    let mixnodes = state.inner.mix_nodes.get_detailed_mixnodes().await;
    let filtered = mixnodes
        .into_iter()
        .filter(|mixnode| mixnode.status == MixnodeStatus::Standby)
        .collect();
    Json(filtered)
}
