use client_core::{
    config::{ClientCoreConfigTrait, GatewayEndpointConfig},
    error::ClientCoreStatusMessage,
};
use futures::{channel::mpsc, StreamExt};
use std::sync::Arc;
use tap::TapFallible;
use task::manager::TaskStatus;
use tokio::sync::RwLock;

use config_common::NymConfig;
use nym_socks5::client::NymClient as Socks5NymClient;
use nym_socks5::client::{config::Config as Socks5Config, Socks5ControlMessageSender};

use crate::{
    error::Result,
    models::{ConnectionStatusKind, ConnectivityTestResult},
    operations,
    state::{GatewayConnectivity, State},
};

pub type ExitStatusReceiver = futures::channel::oneshot::Receiver<Socks5ExitStatusMessage>;

/// Status messages sent by the SOCKS5 client task to the main tauri task.
#[derive(Debug)]
pub enum Socks5ExitStatusMessage {
    /// The SOCKS5 task successfully stopped
    Stopped,
    /// The SOCKS5 task failed to start
    Failed(Box<dyn std::error::Error + Send>),
}

/// The main SOCKS5 client task. It loads the configuration from file determined by the `id`.
pub fn start_nym_socks5_client(
    id: &str,
) -> Result<(
    Socks5ControlMessageSender,
    task::StatusReceiver,
    ExitStatusReceiver,
    GatewayEndpointConfig,
)> {
    log::info!("Loading config from file: {id}");
    let config = Socks5Config::load_from_file(Some(id))
        .tap_err(|_| log::warn!("Failed to load configuration file"))?;
    let used_gateway = config.get_base().get_gateway_endpoint().clone();

    let socks5_client = Socks5NymClient::new(config);
    log::info!("Starting socks5 client");

    // Channel to send control messages to the socks5 client
    let (socks5_ctrl_tx, socks5_ctrl_rx) = mpsc::unbounded();

    // Channel to send status update messages from the background socks5 task to the frontend.
    let (socks5_status_tx, socks5_status_rx) = mpsc::channel(128);

    // Channel to signal back to the main task when the socks5 client finishes, and why
    let (socks5_exit_tx, socks5_exit_rx) = futures::channel::oneshot::channel();

    // Spawn a separate runtime for the socks5 client so we can forcefully terminate.
    // Once we can gracefully shutdown the socks5 client we can get rid of this.
    // The status channel is used to both get the state of the task, and if it's closed, to check
    // for panic.
    std::thread::spawn(|| {
        let result = tokio::runtime::Runtime::new()
            .expect("Failed to create runtime for SOCKS5 client")
            .block_on(async move {
                socks5_client
                    .run_and_listen(socks5_ctrl_rx, socks5_status_tx)
                    .await
            });

        if let Err(err) = result {
            log::error!("SOCKS5 proxy failed: {err}");
            socks5_exit_tx
                .send(Socks5ExitStatusMessage::Failed(err))
                .expect("Failed to send status message back to main task");
            return;
        }

        log::info!("SOCKS5 task finished");
        socks5_exit_tx
            .send(Socks5ExitStatusMessage::Stopped)
            .expect("Failed to send status message back to main task");
    });

    Ok((
        socks5_ctrl_tx,
        socks5_status_rx,
        socks5_exit_rx,
        used_gateway,
    ))
}

#[derive(Clone, serde::Serialize)]
struct Payload {
    title: String,
    message: String,
}

impl Payload {
    fn new(title: String, message: String) -> Self {
        Self { title, message }
    }
}

fn emit_event(event: &str, title: &str, msg: &str, window: &tauri::Window<tauri::Wry>) {
    if let Err(err) = window.emit(event, Payload::new(title.into(), msg.into())) {
        log::error!("Failed to emit tauri event: {err}");
    }
}

fn emit_status_event<T: ToString>(event: &str, msg: &T, window: &tauri::Window<tauri::Wry>) {
    if let Err(err) = window.emit(event, Payload::new("SOCKS5 update".into(), msg.to_string())) {
        log::error!("Failed to emit tauri event: {err}");
    }
}

pub fn start_connection_check(state: Arc<RwLock<State>>, window: tauri::Window<tauri::Wry>) {
    log::debug!("Starting connection check handler");
    tokio::spawn(async move {
        if state.read().await.get_status() != ConnectionStatusKind::Connected {
            log::error!("SOCKS5 connection status check failed: not connected");
            return;
        }

        log::info!("Running connection health check");
        if operations::connection::health_check::run_health_check().await {
            state
                .write()
                .await
                .set_connectivity_test_result(ConnectivityTestResult::Success);
            emit_event(
                "socks5-connection-success-event",
                "SOCKS5 success",
                "SOCKS5 connection health check successful",
                &window,
            );
        } else if state.read().await.get_status() == ConnectionStatusKind::Connected {
            state
                .write()
                .await
                .set_connectivity_test_result(ConnectivityTestResult::Fail);
            log::error!("SOCKS5 connection health check failed");
            emit_event(
                "socks5-connection-fail-event",
                "SOCKS5 error",
                "SOCKS5 connection health check failed",
                &window,
            );
        } else {
            log::debug!("SOCKS5 connection status check cancelled: not connected");
        }

        log::debug!("Connection check handler exiting");
    });
}

async fn handle_task_status(
    task_status: &TaskStatus,
    state: &Arc<RwLock<State>>,
    window: &tauri::Window,
) {
    match task_status {
        TaskStatus::Ready => {
            {
                let mut state_w = state.write().await;
                state_w.mark_connected(window);
            }

            emit_status_event("socks5-connected-event", task_status, window);
            start_connection_check(state.clone(), window.clone());
        }
    }
}

async fn handle_client_status_message(
    client_status_message: &ClientCoreStatusMessage,
    state: &Arc<RwLock<State>>,
    window: &tauri::Window,
) {
    // TODO: use this instead once we change on the frontend too
    let _event_name = match client_status_message {
        ClientCoreStatusMessage::GatewayIsSlow | ClientCoreStatusMessage::GatewayIsVerySlow => {
            "socks5-gateway-status"
        }
    };

    if let Ok(connectivity) = GatewayConnectivity::try_from(client_status_message) {
        state.write().await.set_gateway_connectivity(connectivity);
    }

    emit_status_event("socks5-status-event", client_status_message, window);
}

/// The status listener listens for non-exit status messages from the background socks5 proxy task.
pub fn start_status_listener(
    state: Arc<RwLock<State>>,
    window: tauri::Window<tauri::Wry>,
    mut msg_receiver: task::StatusReceiver,
) {
    log::info!("Starting status listener");

    tokio::spawn(async move {
        while let Some(msg) = msg_receiver.next().await {
            log::info!("SOCKS5 proxy sent status message: {}", msg);

            if let Some(task_status) = msg.downcast_ref::<TaskStatus>() {
                handle_task_status(task_status, &state, &window).await;
            } else if let Some(client_status_message) =
                msg.downcast_ref::<ClientCoreStatusMessage>()
            {
                handle_client_status_message(client_status_message, &state, &window).await;
            } else {
                emit_status_event("socks5-status-event", &msg, &window);
            }
        }
        log::info!("Status listener exiting");
    });
}

/// The disconnect listener listens to the channel setup between the socks5 proxy task and the main
/// tauri task. Primarily it listens for shutdown messages, and updates the state accordingly.
pub fn start_disconnect_listener(
    state: Arc<RwLock<State>>,
    window: tauri::Window<tauri::Wry>,
    exit_status_receiver: ExitStatusReceiver,
) {
    log::trace!("Starting disconnect listener");
    tokio::spawn(async move {
        match exit_status_receiver.await {
            Ok(Socks5ExitStatusMessage::Stopped) => {
                log::info!("SOCKS5 task reported it has finished");
                emit_event(
                    "socks5-event",
                    "SOCKS5 finished",
                    "SOCKS5 task reported it has finished",
                    &window,
                );
            }
            Ok(Socks5ExitStatusMessage::Failed(err)) => {
                log::info!("SOCKS5 task reported error: {err}");
                emit_event(
                    "socks5-event",
                    "SOCKS5 error",
                    &format!("SOCKS5 failed: {err}"),
                    &window,
                );
            }
            Err(_) => {
                log::info!("SOCKS5 task appears to have stopped abruptly");
                emit_event(
                    "socks5-event",
                    "SOCKS5 error",
                    "SOCKS5 stopped abruptly. Please try reconnecting.",
                    &window,
                );
            }
        }

        let mut state_w = state.write().await;
        state_w.mark_disconnected(&window);
    });
}
