use crate::node::node_description::NodeDescription;
use rocket::State;
use rocket_contrib::json::Json;

/// Returns a description of the node and why someone might want to delegate stake to it.
#[get("/description")]
pub(crate) fn description(description: State<NodeDescription>) -> Json<NodeDescription> {
    Json(description.inner().clone())
}
