use futures::SinkExt;
use log::info;
use tauri::Manager;

use nym_socks5::client::{Socks5ControlMessage, Socks5ControlMessageSender};

use crate::{
    config::append_config_id,
    models::{
        AppEventConnectionStatusChangedPayload, ConnectionStatusKind,
        APP_EVENT_CONNECTION_STATUS_CHANGED,
    },
    tasks::{start_nym_socks5_client, StatusReceiver},
};

pub struct State {
    status: ConnectionStatusKind,
    service_provider: Option<String>,
    gateway: Option<String>,
    socks5_client_sender: Option<Socks5ControlMessageSender>,
}

impl State {
    pub fn new() -> Self {
        State {
            status: ConnectionStatusKind::Disconnected,
            service_provider: None,
            gateway: None,
            socks5_client_sender: None,
        }
    }

    #[allow(unused)]
    pub fn get_status(&self) -> ConnectionStatusKind {
        self.status.clone()
    }

    fn set_state(&mut self, status: ConnectionStatusKind, window: &tauri::Window<tauri::Wry>) {
        self.status = status.clone();
        window
            .emit_all(
                APP_EVENT_CONNECTION_STATUS_CHANGED,
                AppEventConnectionStatusChangedPayload { status },
            )
            .unwrap();
    }

    pub fn get_service_provider(&self) -> &Option<String> {
        &self.service_provider
    }

    pub fn set_service_provider(&mut self, provider: String) {
        self.service_provider = Some(provider);
    }

    pub fn get_gateway(&self) -> &Option<String> {
        &self.gateway
    }

    pub fn set_gateway(&mut self, gateway: String) {
        self.gateway = Some(gateway);
    }

    pub async fn init_config(&self) {
        let service_provider = self
            .service_provider
            .as_ref()
            .expect("Attempting to init without service provider");
        let gateway = self
            .gateway
            .as_ref()
            .expect("Attempting to init without gateway");
        crate::config::Config::init(service_provider, gateway).await;
    }

    pub async fn start_connecting(&mut self, window: &tauri::Window<tauri::Wry>) -> StatusReceiver {
        info!("Connecting");
        self.set_state(ConnectionStatusKind::Connecting, window);
        self.status = ConnectionStatusKind::Connecting;

        // Setup configuration by writing to file
        self.init_config().await;

        // Kick off the main task and get the channel for controlling it
        let id = append_config_id(
            self.gateway
                .as_ref()
                .expect("Attempting to start without gateway"),
        );
        let (sender, used_gateway, status_receiver) = start_nym_socks5_client(&id);
        self.gateway = Some(used_gateway.gateway_id);
        self.socks5_client_sender = Some(sender);

        self.status = ConnectionStatusKind::Connected;
        self.set_state(ConnectionStatusKind::Connected, window);

        status_receiver
    }

    pub async fn start_disconnecting(&mut self, window: &tauri::Window<tauri::Wry>) {
        info!("Disconnecting");
        self.set_state(ConnectionStatusKind::Disconnecting, window);
        self.status = ConnectionStatusKind::Disconnecting;

        // Send shutdown message
        if let Some(ref mut sender) = self.socks5_client_sender {
            sender.send(Socks5ControlMessage::Stop).await.unwrap();
        }
    }

    pub async fn mark_disconnected(&mut self, window: &tauri::Window<tauri::Wry>) {
        info!("Disconnected");
        self.status = ConnectionStatusKind::Disconnected;
        self.set_state(ConnectionStatusKind::Disconnected, window);
    }
}
