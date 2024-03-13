use futures::{channel::mpsc, StreamExt};
use nym_client_core::init::types::GatewayDetails;
use nym_client_core::{
    client::base_client::storage::{
        gateway_details::GatewayDetailsStore, MixnetClientStorage, OnDiskPersistent,
    },
    config::TopologyStructure,
    error::ClientCoreStatusMessage,
};
use nym_socks5_client_core::{NymClient as Socks5NymClient, Socks5ControlMessageSender};
use nym_sphinx::params::PacketSize;
use nym_task::manager::TaskStatus;
use nym_topology_control::geo_aware_provider::GroupBy;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;

use crate::{
    config::{Config, PrivacyLevel},
    error::Result,
    events::{self, emit_event, emit_status_event},
    models::{ConnectionStatusKind, ConnectivityTestResult},
    operations,
    state::State,
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

fn override_config_from_env(config: &mut Config, privacy_level: &PrivacyLevel) {
    // Disable both the loop cover traffic that runs in the background as well as the Poisson
    // process that injects cover traffic into the traffic stream.
    if let PrivacyLevel::Medium = privacy_level {
        log::info!("Running in Medium privacy level");
        log::warn!("Disabling cover traffic");
        config.core.base.set_no_cover_traffic_with_keepalive();

        log::warn!("Enabling mixed size packets");
        config
            .core
            .base
            .set_secondary_packet_size(Some(PacketSize::ExtendedPacket16));

        log::warn!("Disabling per-hop delay");
        config.core.base.set_no_per_hop_delays();

        // TODO: selectable in the UI
        let address = config
            .core
            .socks5
            .provider_mix_address
            .parse()
            .expect("failed to parse provider mix address");
        log::warn!("Using geo-aware mixnode selection based on the location of: {address}");
        config
            .core
            .base
            .set_topology_structure(TopologyStructure::GeoAware(GroupBy::NymAddress(address)));
    }
}

/// The main SOCKS5 client task. It loads the configuration from file determined by the `id`.
pub async fn start_nym_socks5_client(
    id: &str,
    privacy_level: &PrivacyLevel,
) -> Result<(
    Socks5ControlMessageSender,
    nym_task::StatusReceiver,
    ExitStatusReceiver,
    GatewayDetails,
)> {
    log::info!("Loading config from file: {id}");
    let mut config = Config::read_from_default_path(id)
        .tap_err(|_| log::warn!("Failed to load configuration file"))?;

    override_config_from_env(&mut config, privacy_level);

    log::trace!("Configuration used: {:#?}", config);

    let storage =
        OnDiskPersistent::from_paths(config.storage_paths.common_paths, &config.core.base.debug)
            .await?;

    let used_gateway = storage
        .gateway_details_store()
        .load_gateway_details()
        .await
        .expect("failed to load gateway details")
        .into();

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
                let socks5_client = Socks5NymClient::new(config.core, storage, None);

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

/// The status listener listens for non-exit status messages from the background socks5 proxy task.
pub fn start_status_listener(
    state: Arc<RwLock<State>>,
    window: tauri::Window<tauri::Wry>,
    mut msg_receiver: nym_task::StatusReceiver,
) {
    log::info!("Starting status listener");

    tokio::spawn(async move {
        while let Some(msg) = msg_receiver.next().await {
            log::info!("SOCKS5 proxy sent status message: {}", msg);

            if let Some(task_status) = msg.downcast_ref::<TaskStatus>() {
                events::handle_task_status(task_status, &state, &window).await;
            } else if let Some(client_status_message) =
                msg.downcast_ref::<ClientCoreStatusMessage>()
            {
                events::handle_client_status_message(client_status_message, &state, &window).await;
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
