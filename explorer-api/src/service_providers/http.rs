use crate::service_providers::models::DirectoryService;
use okapi::openapi3::OpenApi;
use reqwest::Error as ReqwestError;
use rocket::{serde::json::Json, Route};
use rocket_okapi::settings::OpenApiSettings;

static SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

pub fn service_providers_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: get_service_providers]
}

pub async fn get_services() -> Result<Vec<DirectoryService>, ReqwestError> {
    reqwest::get(SERVICE_PROVIDER_WELLKNOWN_URL)
        .await?
        .json::<Vec<DirectoryService>>()
        .await
}

#[openapi(tag = "service_providers")]
#[get("/")]
pub(crate) async fn get_service_providers() -> Json<Vec<DirectoryService>> {
    let result = get_services().await.unwrap();
    Json(result)
}
