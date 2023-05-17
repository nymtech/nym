use actix_web::{get, web, HttpResponse, Responder};
use log::error;

use crate::{
    api::{types::ApiHealth, types::HealthStatus::Healthy, CommandExecutor},
    ephemera_api::{ApiDhtQueryRequest, ApiDhtQueryResponse},
};

#[utoipa::path(
responses(
(status = 200, description = "Endpoint to check if the server is running")),
)]
#[get("/ephemera/node/health")]
#[allow(clippy::unused_async)]
pub(crate) async fn health() -> impl Responder {
    HttpResponse::Ok().json(ApiHealth { status: Healthy })
}

#[utoipa::path(
responses(
(status = 200, description = "Get current broadcast group"),
(status = 500, description = "Server failed to process request")),
)]
#[get("/ephemera/broadcast/group/info")]
pub(crate) async fn broadcast_info(api: web::Data<CommandExecutor>) -> impl Responder {
    match api.get_broadcast_info().await {
        Ok(group) => HttpResponse::Ok().json(group),
        Err(err) => {
            error!("Failed to get current broadcast group: {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "GET block by hash"),
(status = 404, description = "Block not found"),
(status = 500, description = "Server failed to process request")),
params(("hash", description = "Block hash")),
)]
#[get("/ephemera/broadcast/block/{hash}")]
pub(crate) async fn block_by_hash(
    hash: web::Path<String>,
    api: web::Data<CommandExecutor>,
) -> impl Responder {
    match api.get_block_by_id(hash.into_inner()).await {
        Ok(Some(block)) => HttpResponse::Ok().json(block),
        Ok(_) => HttpResponse::NotFound().json("Block not found"),
        Err(err) => {
            error!("Failed to get block by hash: {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "Get block signatures"),
(status = 404, description = "Certificates not found"),
(status = 500, description = "Server failed to process request")),
params(("hash", description = "Block hash")),
)]
#[get("/ephemera/broadcast/block/certificates/{hash}")]
pub(crate) async fn block_certificates(
    hash: web::Path<String>,
    api: web::Data<CommandExecutor>,
) -> impl Responder {
    let id = hash.into_inner();
    match api.get_block_certificates(id.clone()).await {
        Ok(Some(signatures)) => HttpResponse::Ok().json(signatures),
        Ok(_) => HttpResponse::NotFound().json("Certificates not found"),
        Err(err) => {
            error!("Failed to get signatures {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "Get block by height"),
(status = 404, description = "Block not found"),
(status = 500, description = "Server failed to process request")),
params(("height", description = "Block height")),
)]
#[get("/ephemera/broadcast/block/height/{height}")]
pub(crate) async fn block_by_height(
    height: web::Path<u64>,
    api: web::Data<CommandExecutor>,
) -> impl Responder {
    match api.get_block_by_height(height.into_inner()).await {
        Ok(Some(block)) => HttpResponse::Ok().json(block),
        Ok(_) => HttpResponse::NotFound().json("Block not found"),
        Err(err) => {
            error!("Failed to get block {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "Get last block"),
(status = 500, description = "Server failed to process request")),
)]
//Need to use plural(blocks), otherwise overlaps with block_by_id route
#[get("/ephemera/broadcast/blocks/last")]
pub(crate) async fn last_block(api: web::Data<CommandExecutor>) -> impl Responder {
    match api.get_last_block().await {
        Ok(block) => HttpResponse::Ok().json(block),
        Err(err) => {
            error!("Failed to get block {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "Get block broadcast group"),
(status = 404, description = "Block not found"),
(status = 500, description = "Server failed to process request")),
params(("hash", description = "Block hash")),
)]
#[get("/ephemera/broadcast/block/broadcast_info/{hash}")]
pub(crate) async fn block_broadcast_group(
    hash: web::Path<String>,
    api: web::Data<CommandExecutor>,
) -> impl Responder {
    let hash = hash.into_inner();
    match api.get_block_broadcast_info(hash).await {
        Ok(Some(group)) => HttpResponse::Ok().json(group),
        Ok(_) => HttpResponse::NotFound().json("Block not found"),
        Err(err) => {
            error!("Failed to get block broadcast group {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "Get node config"),
(status = 500, description = "Server failed to process request")),
)]
#[get("/ephemera/node/config")]
pub(crate) async fn node_config(api: web::Data<CommandExecutor>) -> impl Responder {
    match api.get_node_config().await {
        Ok(config) => HttpResponse::Ok().json(config),
        Err(err) => {
            error!("Failed to get node config {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}

#[utoipa::path(
responses(
(status = 200, description = "Query dht"),
(status = 500, description = "Server failed to process request")),
params(("query", description = "Dht query")),
)]
#[get("/ephemera/dht/query/{key}")]
pub(crate) async fn query_dht(
    api: web::Data<CommandExecutor>,
    key: web::Path<String>,
) -> impl Responder {
    let key = ApiDhtQueryRequest::parse_key(key.into_inner().as_str());

    match api.query_dht(key).await {
        Ok(Some((key, value))) => {
            let response = ApiDhtQueryResponse::new(key, value);
            HttpResponse::Ok().json(response)
        }
        Ok(_) => HttpResponse::NotFound().json("Not found"),
        Err(err) => {
            error!("Failed to query dht {err}",);
            HttpResponse::InternalServerError().json("Server failed to process request")
        }
    }
}
