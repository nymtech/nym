use futures::channel::mpsc;
use futures::SinkExt;
use log::info;

use config::NymConfig;
#[cfg(not(feature = "coconut"))]
use nym_socks5::client::NymClient as Socks5NymClient;
use nym_socks5::client::{Socks5ControlMessage, Socks5ControlMessageSender};

use crate::config::SOCKS5_CONFIG_ID;
use crate::models::{
    AppEventConnectionStatusChangedPayload, ConnectionStatusKind,
    APP_EVENT_CONNECTION_STATUS_CHANGED,
};
use tauri::Manager;

pub struct State {
    status: ConnectionStatusKind,
    socks5_client_sender: Option<Socks5ControlMessageSender>,
}

impl State {
    pub fn new() -> Self {
        State {
            status: ConnectionStatusKind::Disconnected,
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

    pub async fn init_config() {
        crate::config::Config::init().await;
    }

    pub async fn start_connecting(&mut self, window: &tauri::Window<tauri::Wry>) {
        info!("Connecting");
        self.set_state(ConnectionStatusKind::Connecting, window);
        self.status = ConnectionStatusKind::Connecting;

        // Setup configuration by writing to file
        Self::init_config().await;

        // Kick of the main task and get the channel for controlling it
        let sender = start_nym_socks5_client();
        self.socks5_client_sender = Some(sender);

        self.status = ConnectionStatusKind::Connected;
        self.set_state(ConnectionStatusKind::Connected, window);
    }

    pub async fn start_disconnecting(&mut self, window: &tauri::Window<tauri::Wry>) {
        info!("Disconnecting");
        self.set_state(ConnectionStatusKind::Disconnecting, window);
        self.status = ConnectionStatusKind::Disconnecting;

        // Send shutdown message
        if let Some(ref mut sender) = self.socks5_client_sender {
            sender.send(Socks5ControlMessage::Stop).await.unwrap();
        }

        self.status = ConnectionStatusKind::Disconnected;
        self.set_state(ConnectionStatusKind::Disconnected, window);
    }
}

fn start_nym_socks5_client() -> Socks5ControlMessageSender {
    let id: &str = &SOCKS5_CONFIG_ID;

    info!("Loading config from file");
    let config = nym_socks5::client::config::Config::load_from_file(Some(id)).unwrap();

    let mut socks5_client = Socks5NymClient::new(config);
    info!("Starting socks5 client");

    let (sender, receiver) = mpsc::unbounded();

    // Spawn a separate runtime for the socks5 client so we can forcefully terminate.
    // Once we can gracefully shutdown the socks5 client we can get rid of this.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            socks5_client.run_and_listen(receiver).await;
        });
    });

    sender
}
