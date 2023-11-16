// Copyright (C) 2023 Nym Technologies S.A., GPL-3.0
// Copyright (C) 2022 Mullvad VPN AB, GPL-3.0
use crate::{runtime::create_runtime, shutdown::ShutdownManager};

use std::{
    ffi::OsString,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};
use talpid_types::ErrorExt;
use windows_service::{
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
        ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
};

static SERVICE_NAME: &str = "nymvpnDaemonService";
static SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

lazy_static::lazy_static! {
    static ref SERVICE_ACCESS: ServiceAccess = ServiceAccess::QUERY_CONFIG
    | ServiceAccess::CHANGE_CONFIG
    | ServiceAccess::START
    | ServiceAccess::DELETE;
}

pub fn run() -> Result<(), String> {
    // Start the service dispatcher.
    // This will block current thread until the service stopped and spawn `service_main` on a
    // background thread.
    service_dispatcher::start(SERVICE_NAME, service_main)
        .map_err(|e| e.display_chain_with_msg("Failed to start a service dispatcher"))
}

windows_service::define_windows_service!(service_main, handle_service_main);

pub fn handle_service_main(_arguments: Vec<OsString>) {
    tracing::info!("Service started.");

    let (event_tx, event_rx) = mpsc::channel();

    // Register service event handler
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NO_ERROR even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            ServiceControl::Stop
            | ServiceControl::Preshutdown
            | ServiceControl::PowerEvent(_)
            | ServiceControl::SessionChange(_) => {
                event_tx.send(control_event).unwrap();
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };
    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => handle,
        Err(error) => {
            tracing::error!(
                "{}",
                error.display_chain_with_msg("Failed to register a service control handler")
            );
            return;
        }
    };
    let mut persistent_service_status = PersistentServiceStatus::new(status_handle);
    persistent_service_status
        .set_pending_start(Duration::from_secs(1))
        .unwrap();

    let runtime = create_runtime();
    let runtime = match runtime {
        Err(error) => {
            tracing::error!("{}", error.display_chain());
            persistent_service_status
                .set_stopped(ServiceExitCode::ServiceSpecific(1))
                .unwrap();
            return;
        }
        Ok(runtime) => runtime,
    };

    let shutdown_manager = ShutdownManager::new();

    let result = runtime.block_on(crate::create_daemon(&shutdown_manager));
    let result = if let Ok(daemon) = result {
        let (sc_shutdown_tx, sc_shutdown_rx) = mpsc::channel();
        // Register monitor that translates `ServiceControl` to Daemon events
        start_event_monitor(persistent_service_status.clone(), sc_shutdown_tx, event_rx);

        runtime.block_on(shutdown_manager.register_signal_handler_windows(sc_shutdown_rx));
        persistent_service_status.set_running().unwrap();
        Ok(runtime.block_on(daemon.run()))
    } else {
        result.map(|_| ())
    };

    let exit_code = match result {
        Ok(()) => {
            tracing::info!("Stopping service");
            ServiceExitCode::default()
        }
        Err(error) => {
            tracing::error!("{}", error);
            ServiceExitCode::ServiceSpecific(1)
        }
    };

    persistent_service_status.set_stopped(exit_code).unwrap();
}

/// Start event monitor thread that polls for `ServiceControl` and translates them into calls to
/// Daemon.
fn start_event_monitor(
    persistent_service_status: PersistentServiceStatus,
    sc_shutdown_tx: mpsc::Sender<()>,
    event_rx: mpsc::Receiver<ServiceControl>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut shutdown_handle = ServiceShutdownHandle {
            persistent_service_status,
            sc_shutdown_tx,
        };

        for event in event_rx {
            match event {
                ServiceControl::Stop | ServiceControl::Preshutdown => {
                    // If the daemon is closing due to the system shutting down,
                    // keep blocking traffic after the daemon exits.
                    shutdown_handle.shutdown(event == ServiceControl::Preshutdown);
                }
                _ => (),
            }
        }
    })
}

#[derive(Clone)]
struct ServiceShutdownHandle {
    persistent_service_status: PersistentServiceStatus,
    sc_shutdown_tx: mpsc::Sender<()>,
}

impl ServiceShutdownHandle {
    fn shutdown(&mut self, is_system_shutdown: bool) {
        tracing::info!("is_system_shutdown: {is_system_shutdown}");
        self.persistent_service_status
            .set_pending_stop(Duration::from_secs(10))
            .unwrap();

        if let Err(e) = self.sc_shutdown_tx.send(()) {
            tracing::error!("Failed to send shutdown event to daemon from service: {e}");
        }
    }
}

/// Service status helper with persistent checkpoint counter.
#[derive(Debug, Clone)]
struct PersistentServiceStatus {
    status_handle: ServiceStatusHandle,
    checkpoint_counter: Arc<AtomicUsize>,
}

impl PersistentServiceStatus {
    fn new(status_handle: ServiceStatusHandle) -> Self {
        PersistentServiceStatus {
            status_handle,
            checkpoint_counter: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Tell the system that the service is pending start and provide the time estimate until
    /// initialization is complete.
    fn set_pending_start(&mut self, wait_hint: Duration) -> windows_service::Result<()> {
        self.report_status(
            ServiceState::StartPending,
            wait_hint,
            ServiceExitCode::default(),
        )
    }

    /// Tell the system that the service is running.
    fn set_running(&mut self) -> windows_service::Result<()> {
        self.report_status(
            ServiceState::Running,
            Duration::default(),
            ServiceExitCode::default(),
        )
    }

    /// Tell the system that the service is pending stop and provide the time estimate until the
    /// service is stopped.
    fn set_pending_stop(&mut self, wait_hint: Duration) -> windows_service::Result<()> {
        self.report_status(
            ServiceState::StopPending,
            wait_hint,
            ServiceExitCode::default(),
        )
    }

    /// Tell the system that the service is stopped and provide the exit code.
    fn set_stopped(&mut self, exit_code: ServiceExitCode) -> windows_service::Result<()> {
        self.report_status(ServiceState::Stopped, Duration::default(), exit_code)
    }

    /// Private helper to report the service status update.
    fn report_status(
        &mut self,
        next_state: ServiceState,
        wait_hint: Duration,
        exit_code: ServiceExitCode,
    ) -> windows_service::Result<()> {
        // Automatically bump the checkpoint when updating the pending events to tell the system
        // that the service is making a progress in transition from pending to final state.
        // `wait_hint` should reflect the estimated time for transition to complete.
        let checkpoint = match next_state {
            ServiceState::StartPending
            | ServiceState::StopPending
            | ServiceState::ContinuePending
            | ServiceState::PausePending => self.checkpoint_counter.fetch_add(1, Ordering::SeqCst),
            _ => 0,
        };

        let service_status = ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: next_state,
            controls_accepted: accepted_controls_by_state(next_state),
            exit_code,
            checkpoint: checkpoint as u32,
            wait_hint,
            process_id: None,
        };

        tracing::debug!(
            "Update service status: {:?}, checkpoint: {}, wait_hint: {:?}",
            service_status.current_state,
            service_status.checkpoint,
            service_status.wait_hint
        );

        self.status_handle.set_service_status(service_status)
    }
}

/// Returns the list of accepted service events at each stage of the service lifecycle.
fn accepted_controls_by_state(state: ServiceState) -> ServiceControlAccept {
    let always_accepted = ServiceControlAccept::POWER_EVENT | ServiceControlAccept::SESSION_CHANGE;
    match state {
        ServiceState::StartPending | ServiceState::PausePending | ServiceState::ContinuePending => {
            ServiceControlAccept::empty()
        }
        ServiceState::Running => {
            always_accepted | ServiceControlAccept::STOP | ServiceControlAccept::PRESHUTDOWN
        }
        ServiceState::Paused => {
            always_accepted | ServiceControlAccept::STOP | ServiceControlAccept::PRESHUTDOWN
        }
        ServiceState::StopPending | ServiceState::Stopped => ServiceControlAccept::empty(),
    }
}
