use crate::service_providers::models::{
    DirectoryService, DirectorySpDetailed, HarbourMasterService, PagedResult,
};
use okapi::openapi3::OpenApi;
use reqwest::{Client, Error as ReqwestError};
use rocket::{http::Status, serde::json::Json, Route};
use rocket_okapi::settings::OpenApiSettings;

const SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

const HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";
const HM_SINCE_MIN: u32 = 1000000;
const HM_SIZE: u8 = 100;

#[derive(Debug)]
pub enum GetSpError {
    ReqwestError(ReqwestError),
    Error(String),
}

impl From<ReqwestError> for GetSpError {
    fn from(error: ReqwestError) -> Self {
        GetSpError::ReqwestError(error)
    }
}

impl From<&str> for GetSpError {
    fn from(error: &str) -> Self {
        GetSpError::Error(String::from(error))
    }
}

pub fn service_providers_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: get_service_providers]
}

pub async fn get_services() -> Result<Vec<DirectorySpDetailed>, GetSpError> {
    let reqw = Client::new();

    let services_res = reqw
        .get(SERVICE_PROVIDER_WELLKNOWN_URL)
        .send()
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;

    let directory_sp = services_res
        .iter()
        .find(|item| item.id == "all")
        .ok_or("NymConnect network requesters data not found in response")?;

    let hm_services = reqw
        .get(format!(
            "{HARBOUR_MASTER_URL}?since_min={HM_SINCE_MIN}&size={HM_SIZE}"
        ))
        .send()
        .await?
        .json::<PagedResult<HarbourMasterService>>()
        .await?;

    let sp_list = directory_sp.items.iter().fold(vec![], |mut acc, dir_sp| {
        let directory_sp = hm_services
            .items
            .iter()
            .find(|item| item.service_provider_client_id == dir_sp.address);
        if let Some(sp) = directory_sp {
            acc.push(DirectorySpDetailed {
                id: sp.service_provider_client_id.clone(),
                description: dir_sp.description.clone(),
                address: dir_sp.address.clone(),
                gateway: dir_sp.gateway.clone(),
                routing_score: sp.routing_score,
                service_type: "something".into(),
            });
        }
        acc
    });

    Ok(sp_list)
}

#[openapi(tag = "service_providers")]
#[get("/")]
pub(crate) async fn get_service_providers() -> Result<Json<Vec<DirectorySpDetailed>>, Status> {
    match get_services().await {
        Ok(res) => Ok(Json(res)),
        Err(_) => Err(Status::InternalServerError),
    }
}
