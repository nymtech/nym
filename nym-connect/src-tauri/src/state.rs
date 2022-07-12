use std::time::Duration;

use ::config::NymConfig;
use futures::SinkExt;
use tap::TapFallible;
use tauri::Manager;

use nym_socks5::client::{
    config::Config as Socks5Config, Socks5ControlMessage, Socks5ControlMessageSender,
};

use crate::{
    config::{self, socks5_config_id_appended_with},
    error::{BackendError, Result},
    models::{
        AppEventConnectionStatusChangedPayload, ConnectionStatusKind,
        APP_EVENT_CONNECTION_STATUS_CHANGED,
    },
    tasks::{self, StatusReceiver},
};

pub struct State {
    /// The current connection status
    status: ConnectionStatusKind,

    /// The service provider
    service_provider: Option<String>,

    /// The gateway used. Note that this is also used to create the configuration id
    gateway: Option<String>,

    /// Channel that is used to send command messages to the SOCKS5 client, such as to disconnect
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
        log::info!("{status}");
        self.status = status.clone();
        window
            .emit_all(
                APP_EVENT_CONNECTION_STATUS_CHANGED,
                AppEventConnectionStatusChangedPayload { status },
            )
            .tap_err(|err| log::warn!("{err}"))
            .ok();
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

    /// The effective config id is the static config id appended with the id of the gateway
    pub fn get_config_id(&self) -> Result<String> {
        self.get_gateway()
            .as_ref()
            .ok_or(BackendError::CouldNotGetIdWithoutGateway)
            .and_then(|gateway_id| socks5_config_id_appended_with(gateway_id))
    }

    pub fn load_socks5_config(&self) -> Result<Socks5Config> {
        let id = self.get_config_id()?;
        let config = Socks5Config::load_from_file(Some(&id))
            .tap_err(|_| log::warn!("Failed to load configuration file"))?;
        Ok(config)
    }

    /// Start connecting by first creating a config file, followed by starting a thread running the
    /// SOCKS5 client.
    pub async fn start_connecting(
        &mut self,
        window: &tauri::Window<tauri::Wry>,
    ) -> Result<StatusReceiver> {
        self.set_state(ConnectionStatusKind::Connecting, window);

        // Setup configuration by writing to file
        if let Err(err) = self.init_config().await {
            log::warn!("Failed to initialize: {}", err);

            // Wait a little to give the user some rudimentary feedback that the click actually
            // registered.
            tokio::time::sleep(Duration::from_secs(1)).await;
            self.set_state(ConnectionStatusKind::Disconnected, window);
            return Err(err);
        }

        // Kick off the main task and get the channel for controlling it
        let status_receiver = self.start_nym_socks5_client().await?;
        self.set_state(ConnectionStatusKind::Connected, window);
        Ok(status_receiver)
    }

    /// Create a configuration file
    async fn init_config(&self) -> Result<()> {
        let service_provider = self
            .get_service_provider()
            .as_ref()
            .ok_or(BackendError::CouldNotInitWithoutServiceProvider)?;
        let gateway = self
            .get_gateway()
            .as_ref()
            .ok_or(BackendError::CouldNotInitWithoutGateway)?;
        log::trace!("  service_provider: {:?}", service_provider);
        log::trace!("  gateway: {:?}", gateway);

        config::Config::init(service_provider, gateway).await
    }

    /// Spawn a new thread running the SOCKS5 client
    async fn start_nym_socks5_client(&mut self) -> Result<StatusReceiver> {
        let id = self.get_config_id()?;
        let (control_tx, status_rx, used_gateway) = tasks::start_nym_socks5_client(&id)?;
        self.socks5_client_sender = Some(control_tx);
        self.gateway = Some(used_gateway.gateway_id);
        Ok(status_rx)
    }

    /// Disconnect by sending a message to the SOCKS5 client thread. Once it has finished and is
    /// disconnected, the disconnect handler will mark it as disconnected.
    pub async fn start_disconnecting(&mut self, window: &tauri::Window<tauri::Wry>) -> Result<()> {
        self.set_state(ConnectionStatusKind::Disconnecting, window);

        // Send shutdown message
        match self.socks5_client_sender {
            Some(ref mut sender) => sender
                .send(Socks5ControlMessage::Stop)
                .await
                .map_err(|err| {
                    log::warn!("Failed trying to send disconnect signal: {err}");
                    BackendError::CoundNotSendDisconnectSignal
                }),
            None => {
                log::warn!(
                    "Trying to disconnect without being able to talk to the SOCKS5 client, \
                    is it running?"
                );
                Err(BackendError::CoundNotSendDisconnectSignal)
            }
        }
    }

    /// Once the SOCKS5 client has stopped, this should be called by the disconnect handler to mark
    /// the state as disconnected.
    pub fn mark_disconnected(&mut self, window: &tauri::Window<tauri::Wry>) {
        self.set_state(ConnectionStatusKind::Disconnected, window);
    }
}
