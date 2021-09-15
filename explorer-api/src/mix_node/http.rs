use reqwest::Error as ReqwestError;
use rocket::response::content::Html;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde::Serialize;

use mixnet_contract::{Addr, Coin, Layer, MixNode};

use crate::mix_node::models::{NodeDescription, NodeStats};
use crate::mix_node::templates::{PreviewTemplateData, Templates};
use crate::mix_nodes::{get_mixnode_delegations, Location};
use crate::state::ExplorerApiStateContext;

pub fn mix_node_make_default_routes() -> Vec<Route> {
    routes_with_openapi![get_delegations, get_description, get_stats, list, preview]
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct PrettyMixNodeBondWithLocation {
    pub location: Option<Location>,
    pub bond_amount: Coin,
    pub total_delegation: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub mix_node: MixNode,
}

#[openapi(tag = "mix_node")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyMixNodeBondWithLocation>> {
    Json(
        state
            .inner
            .mix_nodes
            .get()
            .await
            .value
            .values()
            .map(|i| {
                let mix_node = i.bond.clone();
                PrettyMixNodeBondWithLocation {
                    location: i.location.clone(),
                    bond_amount: mix_node.bond_amount,
                    total_delegation: mix_node.total_delegation,
                    owner: mix_node.owner,
                    layer: mix_node.layer,
                    mix_node: mix_node.mix_node,
                }
            })
            .collect::<Vec<PrettyMixNodeBondWithLocation>>(),
    )
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/preview")]
pub(crate) async fn preview(
    pubkey: &str,
    templates: &State<Templates>,
    state: &State<ExplorerApiStateContext>,
) -> Html<String> {
    match get_mixnode_description(pubkey, state).await {
        Some(node_description) => {
            // use handlebars to render an HTML output for an OpenGraph / Twitter preview - this is
            // used in social media apps / messenger apps that show previews for links
            match templates.render_preview(PreviewTemplateData {
                title: node_description.name,
                description: node_description.description,
                url: format!(
                    "https://testnet-milhon-explorer.nymtech.net/nym/mixnodes/{}",
                    pubkey
                ),
                image_url: String::from("https://media2.giphy.com/media/pwyW4XDmtqjG8/200.gif?cid=dda24d50ae15e44c38783edc824618df68645c6af2592b28&amp;rid=200.gif&amp;ct=g"),
            }) {
                Ok(r) => Html(r),
                Err(_e) => Html(String::from("Oh no, something went wrong!")),
            }
        }
        None => Html(String::from("Sorry, mix node not found")),
    }
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/delegations")]
pub(crate) async fn get_delegations(pubkey: &str) -> Json<Vec<mixnet_contract::Delegation>> {
    Json(get_mixnode_delegations(pubkey).await)
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/description")]
pub(crate) async fn get_description(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeDescription>> {
    get_mixnode_description(pubkey, state).await.map(Json)
}

async fn get_mixnode_description(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<NodeDescription> {
    match state
        .inner
        .mix_node_cache
        .clone()
        .get_description(pubkey)
        .await
    {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(cache_value)
        }
        None => {
            trace!("No valid cache value for {}", pubkey);
            match state.inner.get_mix_node(pubkey).await {
                Some(bond) => {
                    match get_mix_node_description(
                        &bond.mix_node.host,
                        &bond.mix_node.http_api_port,
                    )
                    .await
                    {
                        Ok(response) => {
                            // cache the response and return as the HTTP response
                            state
                                .inner
                                .mix_node_cache
                                .set_description(pubkey, response.clone())
                                .await;
                            Some(response)
                        }
                        Err(e) => {
                            error!(
                                "Unable to get description for {} on {}:{} -> {}",
                                pubkey, bond.mix_node.host, bond.mix_node.http_api_port, e
                            );
                            Option::None
                        }
                    }
                }
                None => Option::None,
            }
        }
    }
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/stats")]
pub(crate) async fn get_stats(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeStats>> {
    match state.inner.mix_node_cache.get_node_stats(pubkey).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(Json(cache_value))
        }
        None => {
            trace!("No valid cache value for {}", pubkey);
            match state.inner.get_mix_node(pubkey).await {
                Some(bond) => {
                    match get_mix_node_stats(&bond.mix_node.host, &bond.mix_node.http_api_port)
                        .await
                    {
                        Ok(response) => {
                            // cache the response and return as the HTTP response
                            state
                                .inner
                                .mix_node_cache
                                .set_node_stats(pubkey, response.clone())
                                .await;
                            Some(Json(response))
                        }
                        Err(e) => {
                            error!(
                                "Unable to get description for {} on {}:{} -> {}",
                                pubkey, bond.mix_node.host, bond.mix_node.http_api_port, e
                            );
                            Option::None
                        }
                    }
                }
                None => Option::None,
            }
        }
    }
}

async fn get_mix_node_description(host: &str, port: &u16) -> Result<NodeDescription, ReqwestError> {
    reqwest::get(format!("http://{}:{}/description", host, port))
        .await?
        .json::<NodeDescription>()
        .await
}

async fn get_mix_node_stats(host: &str, port: &u16) -> Result<NodeStats, ReqwestError> {
    reqwest::get(format!("http://{}:{}/stats", host, port))
        .await?
        .json::<NodeStats>()
        .await
}
