use client_core::config::GatewayEndpoint;
use futures::channel::mpsc;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;

use config::NymConfig;
#[cfg(not(feature = "coconut"))]
use nym_socks5::client::NymClient as Socks5NymClient;
use nym_socks5::client::{config::Config as Socks5Config, Socks5ControlMessageSender};

use crate::{error::Result, state::State};

pub type StatusReceiver = futures::channel::oneshot::Receiver<Socks5StatusMessage>;

/// Status messages sent by the SOCKS5 client task to the main tauri task.
#[derive(Debug)]
pub enum Socks5StatusMessage {
    /// The SOCKS5 task successfully stopped
    Stopped,
}

/// The main SOCKS5 client task. It loads the configuration from file determined by the `id`.
pub fn start_nym_socks5_client(
    id: &str,
) -> Result<(Socks5ControlMessageSender, StatusReceiver, GatewayEndpoint)> {
    log::info!("Loading config from file: {id}");
    let config = Socks5Config::load_from_file(Some(id))
        .tap_err(|_| log::warn!("Failed to load configuration file"))?;
    let used_gateway = config.get_base().get_gateway_endpoint().clone();

    let mut socks5_client = Socks5NymClient::new(config);
    log::info!("Starting socks5 client");

    // Channel to send control messages to the socks5 client
    let (socks5_ctrl_tx, socks5_ctrl_rx) = mpsc::unbounded();

    // Channel to signal back to the main task when the socks5 client finishes, and why
    let (socks5_status_tx, socks5_status_rx) = futures::channel::oneshot::channel();

    // Spawn a separate runtime for the socks5 client so we can forcefully terminate.
    // Once we can gracefully shutdown the socks5 client we can get rid of this.
    // The status channel is used to both get the state of the task, and if it's closed, to check
    // for panic.
    std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .expect("Failed to create runtime for SOCKS5 client")
            .block_on(async move { socks5_client.run_and_listen(socks5_ctrl_rx).await });

        log::info!("SOCKS5 task finished");
        socks5_status_tx
            .send(Socks5StatusMessage::Stopped)
            .expect("Failed to send status message back to main task");
    });

    Ok((socks5_ctrl_tx, socks5_status_rx, used_gateway))
}

/// The disconnect listener listens to the channel setup between the socks5 proxy task and the main
/// tauri task. Primarily it listens for shutdown messages, and updates the state accordingly.
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
                // TODO: we should probably generate some events here, or otherwise signal to the
                // frontend.
            }
        }

        let mut state_w = state.write().await;
        state_w.mark_disconnected(&window);
    });
}
