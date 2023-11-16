use daemon::Daemon;
use shutdown::ShutdownManager;
use talpid_core::tunnel_state_machine;
use talpid_types::net::{AllowedEndpoint, Endpoint};
use tokio::sync::oneshot;

pub mod cleanup;
pub mod controller;
pub mod daemon;
pub mod db;
pub mod device;
pub mod location_storage;
pub mod logging;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod runtime;
pub mod shutdown;
pub mod state;
#[cfg(windows)]
pub mod system_service;
pub mod token_storage;
pub mod tunnel;
pub mod unique;
pub mod vpn_session;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::cleanup::remove_old_socket_file;
use tokio_stream::StreamExt;
use nymvpn_config::config;

pub type ResponseTx<T, E> = oneshot::Sender<Result<T, E>>;
pub type AckTx = oneshot::Sender<()>;

use std::{error::Error, path::PathBuf};

use crate::{
    controller::ControllerServer,
    daemon::{DaemonCommandChannel, DaemonEventSender},
    db::Db,
    device::{handler::DeviceHandler, init::initialize_device, storage::DeviceStorage},
    location_storage::LocationStorage,
    token_storage::TokenStorage,
    tunnel::ParameterGenerator,
    unique::already_running,
    vpn_session::{
        handler::VpnSessionHandler, reclaimer::ReclaimerCreator, storage::VpnSessionStorage,
    },
};

fn print_error(e: &dyn Error) {
    tracing::error!("error: {}", e);
    let mut cause = e.source();
    while let Some(e) = cause {
        tracing::error!("caused by: {}", e);
        cause = e.source();
    }
}

pub async fn create_daemon(shutdown_manager: &ShutdownManager) -> Result<Daemon, String> {
    if already_running().await {
        return Err("Nymvpn Daemon is already running".to_owned());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    remove_old_socket_file().await;

    // root user is required otherwise vpn setup will fail
    user_check();

    let db = Db::new().await?;
    // Run DB migrations
    tracing::info!("Running DB migration");
    db.migrate().await?;

    // Initialize device
    let _ = initialize_device(db.connection()).await?;

    let device_handler = DeviceHandler::start(db.connection())
        .await
        .map_err(|e| format!("failed to start device handler: {e:#?}"))?;

    #[cfg(target_os = "macos")]
    let exclusion_gid = {
        macos::bump_filehandle_limit();
        macos::set_exclusion_gid().map_err(|e| format!("failed to set exclusion gid: {e}"))?
    };

    let config = config();
    let daemon_command_channel = DaemonCommandChannel::new();
    let controller_server_and_event_broadcaster = ControllerServer::start(
        daemon_command_channel.sender(),
        shutdown_manager,
        device_handler.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;

    let token_storage = TokenStorage::new(db.connection());
    let device_storage = DeviceStorage::new(db.connection());
    let location_storage = LocationStorage::new(db.connection());
    let vpn_session_storage = VpnSessionStorage::new(db.connection());

    // Reclaim old sessions which were not gracefully ended completely
    vpn_session_storage
        .reclaim()
        .await
        .map_err(|e| format!("failed to reclaim {e}"))?;

    let vpn_session_handler =
        VpnSessionHandler::start(daemon_command_channel.sender().into(), token_storage).await;

    // start reclaimer
    ReclaimerCreator::start(
        vpn_session_storage.clone(),
        device_storage.clone(),
        vpn_session_handler.clone(),
        shutdown_manager.new_shutdown(),
    )
    .await;

    let (offline_state_tx, mut offline_state_rx) = futures_channel::mpsc::unbounded();

    //todo: make better use of offline_state_rx:
    let mut offline_watcher_shutdown = shutdown_manager.new_shutdown();
    tokio::spawn(async move {
        while !offline_watcher_shutdown.is_shutdown() {
            tokio::select! {
                Some(offline) = offline_state_rx.next() => {
                    tracing::info!("Is offline {offline}");
                },
                _  = offline_watcher_shutdown.recv() => {
                    tracing::info!("shutting down offline watcher");
                    break;
                }
            }
        }
    });

    let tunnel_parameters_generator = ParameterGenerator::new(db);

    #[cfg(windows)]
    let exclude_paths = vec![];

    #[cfg(target_os = "windows")]
    let (_volume_update_tx, volume_update_rx) = futures::channel::mpsc::unbounded();

    let allowed_endpoint = AllowedEndpoint {
        #[cfg(windows)]
        clients: vec![std::env::current_exe().expect("daemon executable path not available")],
        endpoint: Endpoint::new(
            config.allowed_endpoint_ipv4().clone(),
            44444,
            talpid_types::net::TransportProtocol::Tcp,
        ),
    };

    let resource_dir: PathBuf = std::env::current_exe()
        .expect("error getting current_exe path")
        .parent()
        .expect("cannot obtain parent path for current_exe")
        .into();

    #[cfg(windows)]
    tracing::info!("Resource dir: {}", resource_dir.display());

    let tunnel_state_machine_handle = tunnel_state_machine::spawn(
        tunnel_state_machine::InitialTunnelState {
            allow_lan: true,
            block_when_disconnected: false,
            dns_servers: Some(vec!["1.1.1.1".parse().unwrap()]),
            allowed_endpoint,
            reset_firewall: true,
            #[cfg(windows)]
            exclude_paths,
        },
        tunnel_parameters_generator,
        Some(config.log_dir().into()),
        resource_dir,
        DaemonEventSender::from(daemon_command_channel.sender()),
        offline_state_tx,
        #[cfg(target_os = "windows")]
        volume_update_rx,
        #[cfg(target_os = "macos")]
        exclusion_gid,
        #[cfg(target_os = "android")]
        None,
        #[cfg(target_os = "linux")]
        tunnel_state_machine::LinuxNetworkingIdentifiers {
            fwmark: nymvpn_types::TUNNEL_FWMARK,
            table_id: nymvpn_types::TUNNEL_TABLE_ID,
        },
    )
    .await
    .map_err(|e| {
        print_error(&e);
        e.to_string()
    })?;
    Ok(Daemon::new(
        daemon_command_channel,
        device_handler,
        vpn_session_storage,
        device_storage,
        vpn_session_handler,
        controller_server_and_event_broadcaster,
        tunnel_state_machine_handle,
        location_storage,
        Some(shutdown_manager.new_shutdown()),
    ))
}

fn user_check() {
    #[cfg(unix)]
    {
        if !nix::unistd::getuid().is_root() {
            tracing::warn!("Running as non-root user, vpn setup will fail")
        }
    }
}
