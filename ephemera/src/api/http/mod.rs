use actix_web::{dev::Server, http::KeepAlive, web::Data, App, HttpServer};
use log::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::CommandExecutor;
use crate::core::builder::NodeInfo;

pub(crate) mod client;
pub(crate) mod query;
pub(crate) mod submit;

/// Starts the HTTP server.
pub(crate) fn init(node_info: &NodeInfo, api: CommandExecutor) -> anyhow::Result<Server> {
    print_startup_messages(node_info);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(api.clone()))
            .service(query::health)
            .service(query::block_by_hash)
            .service(query::block_certificates)
            .service(query::block_by_height)
            .service(query::block_broadcast_group)
            .service(query::last_block)
            .service(query::node_config)
            .service(query::query_dht)
            .service(query::broadcast_info)
            .service(submit::submit_message)
            .service(submit::store_in_dht)
            .service(submit::verify_message_in_block)
            .service(swagger_ui())
    })
    .keep_alive(KeepAlive::Os)
    .bind((node_info.ip.as_str(), node_info.initial_config.http.port))?
    .run();
    Ok(server)
}

/// Builds the Swagger UI.
///
/// Note that all routes you want Swagger docs for must be in the `paths` annotation.
fn swagger_ui() -> SwaggerUi {
    use crate::api::types;
    #[derive(OpenApi)]
    #[openapi(
        paths(
            query::health,
            query::block_by_hash,
            query::block_certificates,
            query::block_by_height,
            query::last_block,
            query::block_broadcast_group,
            query::node_config,
            query::query_dht,
            query::broadcast_info,
            submit::submit_message,
            submit::store_in_dht,
            submit::verify_message_in_block
        ),
        components(schemas(
            types::ApiBlock,
            types::ApiEphemeraMessage,
            types::ApiCertificate,
            types::ApiSignature,
            types::ApiPublicKey,
            types::ApiHealth,
            types::HealthStatus,
            types::ApiEphemeraConfig,
            types::ApiDhtStoreRequest,
            types::ApiDhtQueryRequest,
            types::ApiDhtQueryResponse,
            types::ApiBroadcastInfo,
            types::ApiVerifyMessageInBlock,
        ))
    )]
    struct ApiDoc;
    SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", ApiDoc::openapi())
}

/// Prints messages saying which ports HTTP is running on, and some helpful pointers
/// `OpenAPI` and `Swagger UI` endpoints.
fn print_startup_messages(info: &NodeInfo) {
    let http_root = info.api_address_http();
    info!("Server running on {}", http_root);
    info!("Swagger UI: {}/swagger-ui/", http_root);
    info!("OpenAPI spec is at: {}/api-doc/openapi.json", http_root);
}
