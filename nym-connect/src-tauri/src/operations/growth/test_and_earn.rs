use crate::error::BackendError;
use crate::operations::export::get_identity_key;
use crate::operations::growth::api_client::{
    ClaimPartial, ClientIdPartial, DrawEntry, DrawEntryPartial, DrawWithWordOfTheDay,
    GrowthApiClient, Registration, Winner,
};
use crate::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[cfg(desktop)]
use tauri::api::notification::Notification;
use tauri::Manager;
use tokio::sync::RwLock;

async fn get_client_id(
    state: &tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ClientIdPartial, BackendError> {
    let keypair = get_identity_key(state).await?;
    let client_id = keypair.public_key().to_base58_string();
    let client_id_signature = keypair
        .private_key()
        .sign(client_id.as_bytes())
        .to_base58_string();
    Ok(ClientIdPartial {
        client_id,
        client_id_signature,
    })
}

#[tauri::command]
pub async fn growth_tne_get_client_id(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ClientIdPartial, BackendError> {
    get_client_id(&state).await
}

#[tauri::command]
pub async fn growth_tne_take_part(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Registration, BackendError> {
    let notifications = super::assets::Content::get_notifications();

    let client_id = get_client_id(&state).await?;
    let registration = GrowthApiClient::registrations()
        .register(&client_id)
        .await?;

    log::info!("<<< Test&Earn: registration details: {:?}", registration);

    #[cfg(desktop)]
    if let Err(e) = Notification::new(&app_handle.config().tauri.bundle.identifier)
        .title(notifications.take_part.title)
        .body(notifications.take_part.body)
        .show()
    {
        log::error!("Could not show notification. Error = {:?}", e);
    }

    Ok(registration)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Draws {
    pub current: Option<DrawWithWordOfTheDay>,
    pub next: Option<DrawWithWordOfTheDay>,
    pub draws: Vec<DrawEntry>,
}

#[tauri::command]
pub async fn growth_tne_get_draws(client_details: ClientIdPartial) -> Result<Draws, BackendError> {
    let draws_api = GrowthApiClient::daily_draws();

    let current = draws_api.current().await.ok();
    let next = draws_api.next().await.ok();
    let draws = draws_api.entries(&client_details).await?;

    Ok(Draws {
        current,
        next,
        draws,
    })
}

#[tauri::command]
pub async fn growth_tne_enter_draw(
    client_details: ClientIdPartial,
    draw_id: String,
) -> Result<DrawEntry, BackendError> {
    Ok(GrowthApiClient::daily_draws()
        .enter(&DrawEntryPartial {
            draw_id,
            client_id: client_details.client_id,
            client_id_signature: client_details.client_id_signature,
        })
        .await?)
}

#[tauri::command]
pub async fn growth_tne_submit_wallet_address(
    client_details: ClientIdPartial,
    draw_id: String,
    wallet_address: String,
    registration_id: String,
) -> Result<Winner, BackendError> {
    Ok(GrowthApiClient::daily_draws()
        .claim(&ClaimPartial {
            draw_id,
            client_id: client_details.client_id,
            client_id_signature: client_details.client_id_signature,
            wallet_address,
            registration_id,
        })
        .await?)
}

#[tauri::command]
pub async fn growth_tne_ping(client_details: ClientIdPartial) -> Result<(), BackendError> {
    log::info!("Test&Earn is sending a ping...");
    Ok(GrowthApiClient::registrations()
        .ping(&client_details)
        .await?)
}

#[cfg(desktop)]
#[tauri::command]
pub async fn growth_tne_toggle_window(
    app_handle: tauri::AppHandle,
    window_title: Option<String>,
) -> Result<(), BackendError> {
    if let Some(window) = app_handle.windows().get("growth") {
        log::info!("Closing growth window...");
        if let Err(e) = window.close() {
            log::error!("Unable to close growth window: {:?}", e);
        }
        return Ok(());
    }

    log::info!("Creating growth window...");
    match tauri::WindowBuilder::new(
        &app_handle,
        "growth",
        tauri::WindowUrl::App("growth.html".into()),
    )
    .title(window_title.unwrap_or_else(|| "NymConnect Test&Earn".to_string()))
    .build()
    {
        Ok(window) => {
            if let Err(e) = window.set_focus() {
                log::error!("Unable to focus growth window: {:?}", e);
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Unable to create growth window: {:?}", e);
            Err(BackendError::NewWindowError)
        }
    }
}
