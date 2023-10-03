use std::{sync::Arc, time::Duration};

use log::error;
use rocket::{get, post, serde::json::Json, State};
use tokio::{sync::RwLock, time::sleep};

use crate::{
    error::GatewayHttpApiError,
    node::{Client, ClientRegistry},
};

#[post("/register", data = "<client>")]
pub(crate) async fn register(
    client: Json<Client>,
    clients: &State<Arc<RwLock<ClientRegistry>>>,
) -> Result<(), GatewayHttpApiError> {
    let mut cnt = 0;
    loop {
        cnt += 1;
        match clients.inner().try_write() {
            Ok(mut registry) => {
                registry.insert(client.socket, client.into_inner());
                return Ok(());
            }
            Err(_) => {
                error!("Failed to acquire write lock on client registry");
            }
        }
        if cnt > 3 {
            break;
        }
        sleep(Duration::from_millis(100)).await
    }
    Err(GatewayHttpApiError::ServerError(
        "Failed to acquire write lock on client registry".to_string(),
    ))
}

#[get("/clients")]
pub(crate) async fn clients(
    clients: &State<Arc<RwLock<ClientRegistry>>>,
) -> Result<Json<ClientRegistry>, GatewayHttpApiError> {
    let mut cnt = 0;
    loop {
        cnt += 1;
        match clients.inner().try_read() {
            Ok(registry) => {
                return Ok(Json(registry.clone()));
            }
            Err(_) => {
                error!("Failed to acquire read lock on client registry");
            }
        }
        if cnt > 3 {
            break;
        }
        sleep(Duration::from_millis(100)).await
    }
    Err(GatewayHttpApiError::ServerError(
        "Failed to acquire read lock on client registry".to_string(),
    ))
}

#[get("/client/<pub_key>")]
pub(crate) async fn client(
    pub_key: String,
    clients: &State<Arc<RwLock<ClientRegistry>>>,
) -> Result<Json<Vec<Client>>, GatewayHttpApiError> {
    let mut cnt = 0;
    loop {
        cnt += 1;
        match clients.inner().try_read() {
            Ok(registry) => {
                return Ok(Json(
                    registry
                        .iter()
                        .filter_map(|(_, c)| if c.pub_key == pub_key { Some(c.clone()) } else { None })
                        .collect::<Vec<Client>>(),
                ));
            }
            Err(_) => {
                error!("Failed to acquire read lock on client registry");
            }
        }
        if cnt > 3 {
            break;
        }
        sleep(Duration::from_millis(100)).await
    }
    Err(GatewayHttpApiError::ServerError(
        "Failed to acquire read lock on client registry".to_string(),
    ))
}
