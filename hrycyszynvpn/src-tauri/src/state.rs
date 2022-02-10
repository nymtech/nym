use crate::models::{
  AppEventConnectionStatusChangedPayload, ConnectionStatusKind, APP_EVENT_CONNECTION_STATUS_CHANGED,
};
use tauri::Manager;
use tokio::time::{sleep, Duration};

type StatusChangedHandler = fn(status: &ConnectionStatusKind) -> ();

pub struct State {
  status: ConnectionStatusKind,
}

impl State {
  pub fn new() -> Self {
    State {
      status: ConnectionStatusKind::Disconnected,
    }
  }

  pub fn get_status(&self) -> ConnectionStatusKind {
    self.status.clone()
  }

  fn set_state(&mut self, status: ConnectionStatusKind, window: &tauri::Window<tauri::Wry>) {
    self.status = status.clone();
    window.emit_all(
      APP_EVENT_CONNECTION_STATUS_CHANGED,
      AppEventConnectionStatusChangedPayload { status },
    );
  }

  pub async fn init_config() {
    // crate::config::Config::init().await;
  }

  pub async fn start_connecting(&mut self, window: &tauri::Window<tauri::Wry>) {
    self.set_state(ConnectionStatusKind::Connecting, window);
    sleep(Duration::from_secs(2)).await;
    self.set_state(ConnectionStatusKind::Connected, window);
  }

  pub async fn start_disconnecting(&mut self, window: &tauri::Window<tauri::Wry>) {
    self.set_state(ConnectionStatusKind::Disconnecting, window);
    sleep(Duration::from_secs(2)).await;
    self.set_state(ConnectionStatusKind::Disconnected, window);
  }
}

// use log::{error, info};
//
// use config::NymConfig;
// use nym_client::client::NymClient as NativeNymClient;
// #[cfg(not(feature = "coconut"))]
// use nym_socks5::client::NymClient as Socks5NymClient;
// use nymsphinx::addressing::clients::Recipient;
// use std::sync::Arc;
// use tokio::sync::RwLock;
//
// use crate::config::{NATIVE_CLIENT_CONFIG_ID, SOCKS5_CONFIG_ID};
// use crate::models::ConnectionStatusKind;
//
// pub struct State {
//   status: ConnectionStatusKind,
//   nym_client: Arc<RwLock<Option<NativeNymClient>>>,
//   socks5_client: Arc<RwLock<Option<Socks5NymClient>>>,
//   nym_network_requester: Arc<RwLock<Option<nym_network_requester::core::ServiceProvider>>>,
// }
//
// impl State {
//   pub fn new() -> Self {
//     State {
//       status: ConnectionStatusKind::Disconnected,
//       nym_client: Arc::new(RwLock::new(None)),
//       socks5_client: Arc::new(RwLock::new(None)),
//       nym_network_requester: Arc::new(RwLock::new(None)),
//     }
//   }
//
//   pub fn get_status(&self) -> ConnectionStatusKind {
//     self.status.clone()
//   }
//
//   pub async fn init_config() {
//     crate::config::Config::init().await;
//   }
//
//   pub async fn start_connecting(&mut self) {
//     self.status = ConnectionStatusKind::Connecting;
//
//     let mut nym_client_guard = self.nym_client.write().await;
//     let mut socks5_client_guard = self.socks5_client.write().await;
//     let mut nym_network_requester_guard = self.nym_network_requester.write().await;
//
//     *nym_client_guard = start_nym_native_client().await;
//     if nym_client_guard.is_none() {
//       self.status = ConnectionStatusKind::Disconnected;
//       return;
//     }
//
//     let address = nym_client_guard.unwrap().as_mix_recipient();
//     info!("Nym client address is {}", address);
//
//     *socks5_client_guard = self.start_nym_socks5_client(&address).await;
//     if socks5_client_guard.is_none() {
//       self.status = ConnectionStatusKind::Disconnected;
//       return;
//     }
//
//     *nym_network_requester_guard = self.start_network_requester().await;
//     self.status = ConnectionStatusKind::Connected;
//   }
//
//   pub async fn start_disconnecting(&mut self) {
//     self.status = ConnectionStatusKind::Disconnecting;
//
//     let nym_client_guard = self.nym_client.write().await;
//     let socks5_client_guard = self.socks5_client.write().await;
//     let nym_network_requester_guard = self.nym_network_requester.write().await;
//
//     if nym_network_requester_guard.is_some() {
//       // TODO: implement
//       // nym_network_requester_guard.unwrap().stop().await;
//       *nym_network_requester_guard = None;
//     }
//
//     if socks5_client_guard.is_some() {
//       // TODO: implement
//       // socks5_client_guard.unwrap().stop().await;
//       *socks5_client_guard = None;
//     }
//
//     if nym_client_guard.is_some() {
//       // TODO: implement
//       // nym_client_guard.unwrap().stop().await;
//       *nym_client_guard = None;
//     }
//
//     self.status = ConnectionStatusKind::Disconnected;
//   }
// }
//
// async fn start_nym_native_client() -> Option<NativeNymClient> {
//   let id = NATIVE_CLIENT_CONFIG_ID;
//
//   let config = match nym_client::client::config::Config::load_from_file(Some(id)) {
//     Ok(cfg) => cfg,
//     Err(err) => {
//       error!(
//         "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})",
//         id, err
//       );
//       return None;
//     }
//   };
//
//   // let base_config = config.get_base_mut();
//   // base_config.with_gateway_id("83x9YyNkQ5QEY84ZU6Wmq8XHqfwf9SUtR7g5PAYB1FRY");
//
//   let mut nym_client = NativeNymClient::new(config);
//   nym_client.start().await;
//   Some(nym_client)
// }
//
// async fn start_nym_socks5_client(recipient: &Recipient) -> Option<Socks5NymClient> {
//   let id = SOCKS5_CONFIG_ID;
//
//   let mut config = match nym_socks5::client::config::Config::load_from_file(Some(id)) {
//     Ok(cfg) => cfg,
//     Err(err) => {
//       error!(
//         "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})",
//         id, err
//       );
//       return None;
//     }
//   };
//
//   // let base_config = config.get_base_mut();
//   // base_config.with_gateway_id("83x9YyNkQ5QEY84ZU6Wmq8XHqfwf9SUtR7g5PAYB1FRY");
//
//   config = config.with_provider_mix_address(recipient.to_string());
//
//   let mut socks5_client = Socks5NymClient::new(config);
//   socks5_client.start().await;
//   Some(socks5_client)
// }
//
// // TODO: use remote network requester
// async fn start_network_requester() -> nym_network_requester::core::ServiceProvider {
//   let open_proxy = true;
//   let uri = "ws://localhost:1977";
//   info!("Starting socks5 service provider:");
//   let mut server = nym_network_requester::core::ServiceProvider::new(uri.into(), open_proxy);
//   server.run().await;
//   server
// }
