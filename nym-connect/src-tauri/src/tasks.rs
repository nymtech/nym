use client_core::config::GatewayEndpoint;
use futures::channel::mpsc;
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

use config::NymConfig;
#[cfg(not(feature = "coconut"))]
use nym_socks5::client::NymClient as Socks5NymClient;
use nym_socks5::client::Socks5ControlMessageSender;

use crate::state::State;

pub type StatusReceiver = futures::channel::oneshot::Receiver<Socks5StatusMessage>;

#[derive(Debug)]
pub enum Socks5StatusMessage {
    /// The SOCKS5 task successfully stopped
    Stopped,
}

pub fn start_nym_socks5_client(
    id: &str,
) -> (Socks5ControlMessageSender, GatewayEndpoint, StatusReceiver) {
    info!("Loading config from file: {id}");
    // TODO: handle this gracefully!
    let config = nym_socks5::client::config::Config::load_from_file(Some(id)).unwrap();
    let used_gateway = config.get_base().get_gateway_endpoint().clone();

    let mut socks5_client = Socks5NymClient::new(config);
    info!("Starting socks5 client");

    // Channel to send control messages to the socks5 client
    let (socks5_ctrl_tx, socks5_ctrl_rx) = mpsc::unbounded();

    // Channel to signal back to the main task when the socks5 client finishes, and why
    let (socks5_status_tx, socks5_status_rx) = futures::channel::oneshot::channel();

    // Spawn a separate runtime for the socks5 client so we can forcefully terminate.
    // Once we can gracefully shutdown the socks5 client we can get rid of this.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            socks5_client.run_and_listen(socks5_ctrl_rx).await;
        });

        log::info!("SOCKS5 task finished");
        socks5_status_tx.send(Socks5StatusMessage::Stopped).unwrap();
    });

    (socks5_ctrl_tx, used_gateway, socks5_status_rx)
}

pub fn start_disconnect_listener(
    state: Arc<RwLock<State>>,
    window: tauri::Window<tauri::Wry>,
    status_receiver: StatusReceiver,
) {
    log::trace!("Starting disconnect listener");
    tokio::spawn(async move {
        match status_receiver.await {
            Ok(Socks5StatusMessage::Stopped) => {
                log::info!("SOCKS5 task reported it has finished");
            }
            Err(_) => {
                log::info!("SOCKS5 task appears to have stopped abruptly");
            }
        }

        let mut state_w = state.write().await;
        state_w.mark_disconnected(&window).await;
    });
}
