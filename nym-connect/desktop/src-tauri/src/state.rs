use futures::SinkExt;
use log::error;
use nym_client_core::error::ClientCoreStatusMessage;
use nym_socks5_client_core::{Socks5ControlMessage, Socks5ControlMessageSender};
use std::time::Duration;
use tap::TapFallible;
use tauri::Manager;
use tokio::time::Instant;

use crate::config::Config;
use crate::config::PrivacyLevel;
use crate::config::SelectedGateway;
use crate::config::SelectedSp;
use crate::config::UserData;
use crate::{
    config::{self, socks5_config_id_appended_with},
    error::{BackendError, Result},
    models::{
        AppEventConnectionStatusChangedPayload, ConnectionStatusKind, ConnectivityTestResult,
        APP_EVENT_CONNECTION_STATUS_CHANGED,
    },
    tasks::{self, ExitStatusReceiver},
};

// The client will emit messages if the connection to the gateway is poor (or the gateway can't
// keep up with the messages we are sending). If no messages about this has been received for a
// certain duration then we assume it's all good.
const GATEWAY_CONNECTIVITY_TIMEOUT_SECS: u64 = 20;

#[derive(Clone, Copy, Debug)]
pub enum GatewayConnectivity {
    Good,
    Bad { when: Instant },
    VeryBad { when: Instant },
}

impl TryFrom<&ClientCoreStatusMessage> for GatewayConnectivity {
    type Error = BackendError;

    fn try_from(value: &ClientCoreStatusMessage) -> Result<Self, Self::Error> {
        let conn = match value {
            ClientCoreStatusMessage::GatewayIsSlow => GatewayConnectivity::Bad {
                when: Instant::now(),
            },
            ClientCoreStatusMessage::GatewayIsVerySlow => GatewayConnectivity::VeryBad {
                when: Instant::now(),
            },
        };
        Ok(conn)
    }
}

#[derive(Debug)]
pub struct State {
    /// The current connection status
    status: ConnectionStatusKind,

    /// The service provider
    service_provider: Option<String>,

    /// The gateway used. Note that this is also used to create the configuration id
    gateway: Option<String>,

    /// Channel that is used to send command messages to the SOCKS5 client, such as to disconnect
    socks5_client_sender: Option<Socks5ControlMessageSender>,

    /// The client will periodically report connectivity to the gateway it's connected to. Unless
    /// we get a status message from the client we assume it's good.
    gateway_connectivity: GatewayConnectivity,

    /// The latest end-to-end connectivity test result. The first test is initiated on connection
    /// established. Additional tests can be triggered.
    connectivity_test_result: ConnectivityTestResult,

    /// User data saved on disk, like user settings
    user_data: UserData,
}

impl State {
    pub fn new(user_data: UserData) -> Self {
        State {
            status: ConnectionStatusKind::Disconnected,
            service_provider: None,
            gateway: None,
            socks5_client_sender: None,
            gateway_connectivity: GatewayConnectivity::Good,
            connectivity_test_result: ConnectivityTestResult::NotAvailable,
            user_data,
        }
    }

    pub fn get_gateway_connectivity(&mut self) -> GatewayConnectivity {
        self.gateway_connectivity = match self.gateway_connectivity {
            c @ (GatewayConnectivity::Bad { when } | GatewayConnectivity::VeryBad { when }) => {
                if Instant::now() > when + Duration::from_secs(GATEWAY_CONNECTIVITY_TIMEOUT_SECS) {
                    GatewayConnectivity::Good
                } else {
                    c
                }
            }
            current => current,
        };
        self.gateway_connectivity
    }

    pub fn get_user_data(&self) -> &UserData {
        &self.user_data
    }

    pub fn clear_user_data(&mut self) -> Result<()> {
        self.user_data.clear().map_err(|e| {
            error!("Failed to clear user data {e}");
            BackendError::UserDataWriteError
        })
    }

    pub fn set_monitoring(&mut self, enabled: bool) -> Result<()> {
        self.user_data.monitoring = Some(enabled);
        self.user_data.write().map_err(|e| {
            error!("Failed to write user data to disk {e}");
            BackendError::UserDataWriteError
        })
    }

    pub fn set_privacy_level(&mut self, privacy_level: PrivacyLevel) -> Result<()> {
        self.user_data.privacy_level = Some(privacy_level);
        self.user_data.write().map_err(|e| {
            error!("Failed to write user data to disk {e}");
            BackendError::UserDataWriteError
        })
    }

    pub fn set_user_selected_gateway(&mut self, gateway: Option<SelectedGateway>) -> Result<()> {
        self.user_data.selected_gateway = gateway;
        self.user_data.write().map_err(|e| {
            error!("Failed to write user data to disk {e}");
            BackendError::UserDataWriteError
        })
    }

    pub fn set_user_selected_sp(&mut self, service_provider: Option<SelectedSp>) -> Result<()> {
        self.user_data.selected_sp = service_provider;
        self.user_data.write().map_err(|e| {
            error!("Failed to write user data to disk {e}");
            BackendError::UserDataWriteError
        })
    }

    pub fn set_gateway_connectivity(&mut self, gateway_connectivity: GatewayConnectivity) {
        self.gateway_connectivity = gateway_connectivity
    }

    pub fn get_connectivity_test_result(&self) -> ConnectivityTestResult {
        self.connectivity_test_result
    }

    pub fn set_connectivity_test_result(
        &mut self,
        connectivity_test_result: ConnectivityTestResult,
    ) {
        self.connectivity_test_result = connectivity_test_result;
    }

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

    pub fn set_service_provider(&mut self, provider: Option<String>) {
        self.service_provider = provider;
    }

    pub fn get_gateway(&self) -> &Option<String> {
        &self.gateway
    }

    pub fn set_gateway(&mut self, gateway: Option<String>) {
        self.gateway = gateway;
    }

    /// The effective config id is the static config id appended with the id of the gateway
    pub fn get_config_id(&self) -> Result<String> {
        let gateway_id = self
            .get_gateway()
            .as_ref()
            .ok_or(BackendError::CouldNotGetIdWithoutGateway)?;
        Ok(socks5_config_id_appended_with(gateway_id))
    }

    pub fn load_config(&self) -> Result<Config> {
        let id = self.get_config_id()?;
        let config = Config::read_from_default_path(id)
            .tap_err(|_| log::warn!("Failed to load configuration file"))?;
        Ok(config)
    }

    /// Start connecting by first creating a config file, followed by starting a thread running the
    /// SOCKS5 client.
    pub async fn start_connecting(
        &mut self,
        window: &tauri::Window<tauri::Wry>,
    ) -> Result<(nym_task::StatusReceiver, ExitStatusReceiver)> {
        self.set_state(ConnectionStatusKind::Connecting, window);

        // Setup configuration by writing to file
        if let Err(err) = self.init_config().await {
            log::error!("Failed to initialize: {err}");

            // Wait a little to give the user some rudimentary feedback that the click actually
            // registered.
            tokio::time::sleep(Duration::from_secs(1)).await;
            self.set_state(ConnectionStatusKind::Disconnected, window);
            return Err(err);
        }

        // Kick off the main task and get the channel for controlling it
        self.start_nym_socks5_client().await
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
    async fn start_nym_socks5_client(
        &mut self,
    ) -> Result<(nym_task::StatusReceiver, ExitStatusReceiver)> {
        let id = self.get_config_id()?;
        let privacy_level = self.user_data.privacy_level.unwrap_or_default();
        let (control_tx, msg_rx, exit_status_rx, used_gateway) =
            tasks::start_nym_socks5_client(&id, &privacy_level).await?;
        self.socks5_client_sender = Some(control_tx);
        self.gateway = Some(used_gateway.gateway_id().to_base58_string());
        Ok((msg_rx, exit_status_rx))
    }

    /// Once the SOCKS5 client is operational, the status listener would call this
    pub fn mark_connected(&mut self, window: &tauri::Window<tauri::Wry>) {
        log::trace!("state::mark_connected");
        self.set_state(ConnectionStatusKind::Connected, window);
    }

    /// Disconnect by sending a message to the SOCKS5 client thread. Once it has finished and is
    /// disconnected, the disconnect handler will mark it as disconnected.
    pub async fn start_disconnecting(&mut self, window: &tauri::Window<tauri::Wry>) -> Result<()> {
        log::trace!("state::start_disconnecting");
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
