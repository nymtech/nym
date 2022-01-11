use log::info;
use rocket::http::Method;
use rocket::{Build, Request, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use rocket_okapi::swagger_ui::make_swagger_ui;

use crate::country_statistics::http::country_statistics_make_default_routes;
use crate::http::swagger::get_docs;
use crate::mix_node::http::mix_node_make_default_routes;
use crate::mix_nodes::http::mix_nodes_make_default_routes;
use crate::overview::http::overview_make_default_routes;
use crate::ping::http::ping_make_default_routes;
use crate::state::ExplorerApiStateContext;

mod swagger;

pub(crate) fn start(state: ExplorerApiStateContext) {
    tokio::spawn(async move {
        info!("Starting up...");
        configure_rocket(state).launch().await
    });
}

fn configure_rocket(state: ExplorerApiStateContext) -> Rocket<Build> {
    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["*"]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .unwrap();

    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let config = rocket::config::Config::release_default();
    let mut building_rocket = rocket::build().configure(config);

    mount_endpoints_and_merged_docs! {
        building_rocket,
        "/v1".to_owned(),
        openapi_settings,
        "/ping" => ping_make_default_routes(&openapi_settings),
        "/countries" => country_statistics_make_default_routes(&openapi_settings),
        "/mix-node" => mix_node_make_default_routes(&openapi_settings),
        "/mix-nodes" => mix_nodes_make_default_routes(&openapi_settings),
        "/overview" => overview_make_default_routes(&openapi_settings),
    };

    building_rocket
        .mount("/swagger", make_swagger_ui(&get_docs()))
        .register("/", catchers![not_found])
        .manage(state)
        .attach(cors)
}

#[catch(404)]
pub(crate) fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}
