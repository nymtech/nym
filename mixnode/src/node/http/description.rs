use crate::node::node_description::NodeDescription;
use axum::extract::Query;
use nym_node::http::api::{FormattedResponse, OutputParams};
use rocket::serde::json::Json;
use rocket::State;

/// Returns a description of the node and why someone might want to delegate stake to it.
#[get("/description")]
pub(crate) fn description(description: &State<NodeDescription>) -> Json<NodeDescription> {
    Json(description.inner().clone())
}

/// Returns a description of the node and why someone might want to delegate stake to it.
pub(crate) async fn description_axum(
    description: NodeDescription,
    Query(output): Query<OutputParams>,
) -> MixnodeDescriptionResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(description)
}

pub type MixnodeDescriptionResponse = FormattedResponse<NodeDescription>;
