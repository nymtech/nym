use log::info;

use config::NymConfig;
#[cfg(not(feature = "coconut"))]
use nym_socks5::client::NymClient as Socks5NymClient;

use crate::config::SOCKS5_CONFIG_ID;
use crate::models::{
  AppEventConnectionStatusChangedPayload, ConnectionStatusKind, APP_EVENT_CONNECTION_STATUS_CHANGED,
};
use tauri::Manager;

pub struct State {
  status: ConnectionStatusKind,
  //socks5_client: Arc<RwLock<Option<Socks5NymClient>>>,
}

impl State {
  pub fn new() -> Self {
    State {
      status: ConnectionStatusKind::Disconnected,
      //socks5_client: Arc::new(RwLock::new(None)),
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

    Self::init_config().await;
    tokio::spawn(async move { start_nym_socks5_client().await });

    self.status = ConnectionStatusKind::Connected;
    self.set_state(ConnectionStatusKind::Connected, window);
  }

  pub async fn start_disconnecting(&mut self, window: &tauri::Window<tauri::Wry>) {
    info!("Disconnecting");
    self.set_state(ConnectionStatusKind::Disconnecting, window);
    self.status = ConnectionStatusKind::Disconnecting;

    // TODO: implement
    // socks5_client_guard.unwrap().stop().await;
    // *socks5_client_guard = None;

    self.status = ConnectionStatusKind::Disconnected;
    self.set_state(ConnectionStatusKind::Disconnected, window);
  }
}

async fn start_nym_socks5_client() {
  let id = SOCKS5_CONFIG_ID;

  info!("Loading config from file");
  let config = nym_socks5::client::config::Config::load_from_file(Some(id)).unwrap();

  let mut socks5_client = Socks5NymClient::new(config);
  info!("Starting socks5 client");
  socks5_client.start().await;
}
