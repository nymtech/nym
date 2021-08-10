use log::info;
use rocket_okapi::swagger_ui::make_swagger_ui;

use crate::country_statistics::http::country_statistics_make_default_routes;
use crate::http::cors::Cors;
use crate::http::swagger::get_docs;
use crate::mix_node::http::mix_node_make_default_routes;
use crate::ping::http::ping_make_default_routes;
use crate::state::ExplorerApiStateContext;
use rocket::Request;

mod cors;
mod swagger;

pub(crate) fn start(state: ExplorerApiStateContext) {
    tokio::spawn(async move {
        info!("Starting up...");

        let config = rocket::config::Config::release_default();
        rocket::build()
            .configure(config)
            .mount("/countries", country_statistics_make_default_routes())
            .mount("/ping", ping_make_default_routes())
            .mount("/mix-node", mix_node_make_default_routes())
            .mount("/swagger", make_swagger_ui(&get_docs()))
            .register("/", catchers![not_found])
            .manage(state)
            .attach(Cors)
            .launch()
            .await
    });
}

#[catch(404)]
pub(crate) fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}
