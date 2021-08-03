mod swagger;

use crate::country_statistics::http::make_default_routes;
use crate::http::swagger::get_docs;
use crate::ping::http::ping_make_default_routes;
use crate::state::ExplorerApiStateContext;
use log::info;
use rocket_okapi::swagger_ui::make_swagger_ui;

pub(crate) fn start(state: ExplorerApiStateContext) {
    tokio::spawn(async move {
        info!("Starting up...");

        let config = rocket::config::Config::release_default();

        rocket::build()
            .configure(config)
            .mount("/countries", make_default_routes())
            .mount("/ping", ping_make_default_routes())
            .mount("/swagger", make_swagger_ui(&get_docs()))
            // .register("/", catchers![not_found])
            .manage(state)
            // .manage(descriptor)
            // .manage(node_stats_pointer)
            .launch()
            .await
    });
}
