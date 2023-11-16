use std::fmt::Display;

use talpid_core::tunnel_state_machine::{TunnelCommand, TunnelStateMachineHandle};
use talpid_types::tunnel::TunnelStateTransition;
use tokio::sync::oneshot;
use nymvpn_controller::auth::Auth;
use nymvpn_migration::DbErr;
use nymvpn_types::{
    location::Location,
    notification::Notification,
    nymvpn_server::{ClientConnected, EndSession, NewSession, UserCredentials, VpnSessionStatus},
    vpn_session::VpnStatus,
};

use crate::{
    controller::ControllerServerAndEventBroadcaster,
    device::{handler::DeviceHandler, storage::DeviceStorage, DeviceError},
    location_storage::LocationStorage,
    shutdown::Shutdown,
    state::DaemonState,
    vpn_session::{
        handler::{VpnSessionError, VpnSessionHandler},
        storage::{SessionInfo, VpnSessionStorage},
    },
    ResponseTx,
};

pub struct Daemon {
    // in memory current state of the server + client side state
    state: DaemonState,
    controller_server_and_event_broadcaster: ControllerServerAndEventBroadcaster,
    daemon_command_sender: DaemonCommandSender,
    daemon_receiver: DaemonReceiver,
    device_handler: DeviceHandler,
    vpn_session_storage: VpnSessionStorage,
    device_storage: DeviceStorage,
    vpn_session_handler: VpnSessionHandler,
    tunnel_state_machine_handle: TunnelStateMachineHandle,
    location_storage: LocationStorage,
    shutdown: Option<Shutdown>,
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("daemon is offline")]
    DaemonUnavailable,
    #[error("another vpn session in progress: {0}")]
    AnotherVpnSessionInProgress(Location),
    #[error("invalid op when vpn session in progress: {0}")]
    InvalidOpVpnSessionInProgress(String),
    #[error("")]
    DbErr(#[from] DbErr),
    #[error("device error: {0}")]
    DeviceError(#[from] DeviceError),
    #[error("vpn session error: {0}")]
    VpnSessionError(#[from] VpnSessionError),
}

#[derive(Debug)]
pub enum DaemonCommand {
    IsAuthenticated(ResponseTx<bool, DaemonError>),
    AccountSignIn(ResponseTx<(), DaemonError>, UserCredentials),
    AccountSignOut(ResponseTx<(), DaemonError>),
    ListLocations(ResponseTx<Vec<Location>, DaemonError>),
    RecentLocations(ResponseTx<Vec<Location>, DaemonError>),
    Connect(ResponseTx<VpnStatus, DaemonError>, Location),
    Disconnect(ResponseTx<VpnStatus, DaemonError>),
    GetVpnStatus(ResponseTx<VpnStatus, DaemonError>),
    GetNotifications(ResponseTx<Vec<Notification>, DaemonError>),
    AckNotification(ResponseTx<(), DaemonError>, String),
    LatestAppVersion(ResponseTx<String, DaemonError>),
}

pub type DaemonReceiver = tokio::sync::mpsc::UnboundedReceiver<DaemonEvent>;
pub type DaemonSender = tokio::sync::mpsc::UnboundedSender<DaemonEvent>;

pub struct DaemonCommandChannel {
    sender: DaemonCommandSender,
    receiver: DaemonReceiver,
}

impl DaemonCommandChannel {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sender: DaemonCommandSender(tx),
            receiver: rx,
        }
    }

    pub fn sender(&self) -> DaemonCommandSender {
        self.sender.clone()
    }

    pub fn destructure(self) -> (DaemonCommandSender, DaemonReceiver) {
        (self.sender, self.receiver)
    }
}

#[derive(Clone)]
pub struct DaemonCommandSender(DaemonSender);

#[derive(Clone)]
pub struct DaemonEventSender(DaemonSender);

impl DaemonCommandSender {
    pub fn send(&self, command: DaemonCommand) -> Result<(), DaemonError> {
        self.0
            .send(DaemonEvent::Command(command))
            .map_err(|_| DaemonError::DaemonUnavailable)
    }
}

impl DaemonEventSender {
    pub fn send(&self, event: DaemonEvent) -> Result<(), DaemonError> {
        self.0
            .send(event)
            .map_err(|_| DaemonError::DaemonUnavailable)
    }
}

impl<E> talpid_core::mpsc::Sender<E> for DaemonEventSender
where
    DaemonEvent: From<E>,
{
    fn send(&self, event: E) -> Result<(), talpid_core::mpsc::Error> {
        self.0
            .send(DaemonEvent::from(event))
            .map_err(|_| talpid_core::mpsc::Error::ChannelClosed)
    }
}

impl From<DaemonCommandSender> for DaemonEventSender {
    fn from(dcs: DaemonCommandSender) -> Self {
        Self(dcs.0)
    }
}

/// All possible events that can happen during the lifetime of a Daemon
#[derive(Debug)]
pub enum DaemonEvent {
    /// Command for the Daemon
    Command(DaemonCommand),
    /// Initiated by signals like ctrl-c, SIGINT or SIGTERM
    Shutdown,
    /// Vpn Session Status received from Server
    VpnSessionStatus(VpnSessionStatus),
    /// Tunnel has changed state.
    TunnelStateTransition(TunnelStateTransition),
}

impl From<TunnelStateTransition> for DaemonEvent {
    fn from(tst: TunnelStateTransition) -> Self {
        DaemonEvent::TunnelStateTransition(tst)
    }
}

impl Display for DaemonEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let event = match self {
            DaemonEvent::Command(command) => match command {
                DaemonCommand::IsAuthenticated(_) => "IsAuthenticated".into(),
                DaemonCommand::AccountSignIn(_, user_creds) => {
                    format!("AccountSignIn: {}", user_creds.email)
                }
                DaemonCommand::AccountSignOut(_) => "AccountSignOut".into(),
                DaemonCommand::ListLocations(_) => "ListLocations".into(),
                DaemonCommand::RecentLocations(_) => "RecentLocations".into(),
                DaemonCommand::Connect(_, location) => format!("Connect: {}", location.code),
                DaemonCommand::Disconnect(_) => "Disconnect".into(),
                DaemonCommand::GetVpnStatus(_) => "GetVpnStatus".into(),
                DaemonCommand::GetNotifications(_) => "GetNotifications".into(),
                DaemonCommand::AckNotification(_, id) => format!("AckNotification: {id}"),
                DaemonCommand::LatestAppVersion(_) => "LatestAppVersion".into(),
            },
            DaemonEvent::Shutdown => "Shutdown".into(),
            DaemonEvent::VpnSessionStatus(status) => format!("VpnSessionStatus: {status}"),
            DaemonEvent::TunnelStateTransition(transition) => match transition {
                TunnelStateTransition::Disconnected => "TunnelStateTransition: Disconnected".into(),
                TunnelStateTransition::Connecting(endpoint) => {
                    format!("TunnelStateTransition: {endpoint}")
                }
                TunnelStateTransition::Connected(endpoint) => {
                    format!("TunnelStateTransition: {endpoint}")
                }
                TunnelStateTransition::Disconnecting(action) => {
                    format!("TunnelStateTransition: {action:?}")
                }
                TunnelStateTransition::Error(e) => format!("TunnelStateTransition: {e:?}"),
            },
        };

        write!(f, "{event}")
    }
}

#[async_trait::async_trait]
pub trait EventListener {
    async fn send_vpn_status(&self, status: VpnStatus);

    async fn send_notification(&self, notification: Notification);

    //todo: add other events
}

impl Daemon {
    pub fn new(
        dc: DaemonCommandChannel,
        device_handler: DeviceHandler,
        vpn_session_storage: VpnSessionStorage,
        device_storage: DeviceStorage,
        vpn_session_handler: VpnSessionHandler,
        controller_server_and_event_broadcaster: ControllerServerAndEventBroadcaster,
        tunnel_state_machine_handle: TunnelStateMachineHandle,
        location_storage: LocationStorage,
        shutdown: Option<Shutdown>,
    ) -> Self {
        let (daemon_command_sender, daemon_receiver) = dc.destructure();

        Daemon {
            state: DaemonState::new(),
            daemon_command_sender,
            daemon_receiver,
            device_handler,
            device_storage,
            vpn_session_storage,
            vpn_session_handler,
            controller_server_and_event_broadcaster,
            tunnel_state_machine_handle,
            location_storage,
            shutdown,
        }
    }

    fn register_shutdown(&mut self) {
        let mut shutdown = self.shutdown.take().unwrap();
        let sender = DaemonEventSender::from(self.daemon_command_sender.clone());
        tokio::spawn(async move {
            shutdown.recv().await;
            if let Err(e) = sender.send(DaemonEvent::Shutdown) {
                //todo: Display trait
                tracing::error!("failed to send shutdown event to Daemon: {e:#?}");
            }
        });
    }

    pub async fn run(mut self) {
        self.register_shutdown();
        while let Some(event) = self.daemon_receiver.recv().await {
            if let DaemonEvent::Shutdown = event {
                break;
            }
            self.handle_event(event).await;
        }
        self.handle_shutdown().await;
    }

    async fn handle_shutdown(mut self) {
        tracing::info!("handling shutdown ...");

        // if any, disconnect and end existing session
        if let Err(err) = self.on_disconnect_inner("daemon shutdown".into()).await {
            tracing::error!("when ending session during shutdown: {err}");
        };

        // wait for tunnel state machine to stop
        self.tunnel_state_machine_handle.try_join().await;

        if let Err(err) = self.vpn_session_handler.shutdown().await {
            tracing::error!("error when vpn session handler was shutting down: {err}");
        };

        let ControllerServerAndEventBroadcaster {
            events_subscribers,
            controller_server_handle,
        } = self.controller_server_and_event_broadcaster;

        let mut guard = events_subscribers.write().await;
        guard.clear();

        drop(guard);
        drop(events_subscribers);

        let _ = tokio::join!(controller_server_handle);
        // device handler last as controller depends on it for auth
        if let Err(err) = self.device_handler.shutdown().await {
            tracing::error!("error when device handler was shutting down: {err}");
        };
    }

    async fn handle_event(&mut self, event: DaemonEvent) {
        tracing::debug!("daemon event: {event}");
        match event {
            DaemonEvent::Command(command) => self.handle_command(command).await,
            DaemonEvent::VpnSessionStatus(vpn_session_status) => {
                self.handle_vpn_session_status(vpn_session_status).await
            }
            DaemonEvent::TunnelStateTransition(transition) => {
                self.handle_tunnel_state_transition(transition).await
            }
            DaemonEvent::Shutdown => {}
        }
    }

    async fn handle_tunnel_state_transition(&mut self, transition: TunnelStateTransition) {
        tracing::info!("tunnel transition: {transition:?}");
        let processed = self
            .vpn_session_storage
            .tunnel_state_transition(transition.clone(), self.state.vpn_status())
            .await
            .expect("failed to process tunnel state transition");

        if let Some(reason) = processed.end_session {
            tracing::info!("ending session after tunnel transition {reason}");
            if let Err(err) = self.end_session(reason).await {
                tracing::error!(
                    "failed to end session on state transition: {transition:?}: {err:?}"
                );
            };
        }

        if let Some(session_info) = processed.client_connected {
            tracing::info!("client connected {}", session_info.request_id);
            self.client_connected(session_info).await;
        }

        if let Some(tunnel_command) = processed.tunnel_command {
            tracing::info!("sending tunnel command after tunnel state transition");
            self.send_tunnel_command(tunnel_command);
        }

        if let Some(notification) = processed.notification {
            tracing::info!("sending notification after tunnel state transition {notification:?}");
            self.add_notification(notification).await;
        }

        // update vpn status and send device event
        self.set_vpn_status(processed.vpn_status).await;
    }

    async fn handle_vpn_session_status(&mut self, vpn_session_status: VpnSessionStatus) {
        // Update DB and get next set of actions
        let processed = self
            .vpn_session_storage
            .updated_server_status(vpn_session_status.clone())
            .await
            .map_err(|e| {
                tracing::error!(
                    "failed to process updated vpn session status {vpn_session_status}: {e}"
                )
            })
            .expect("unrecoverable db error in handle_vpn_session_status");

        if let Some(notification) = processed.notification {
            self.add_notification(notification).await;
        }

        if let Some(new_vpn_status) = processed.vpn_status {
            // Update in memory status and notify clients of new status
            self.set_vpn_status(new_vpn_status.clone()).await;

            // Start next state machine if this is ServerReady
            self.update_tunnel_on_new_status(new_vpn_status).await;
        }
    }

    async fn update_tunnel_on_new_status(&self, new_vpn_status: VpnStatus) {
        if let VpnStatus::ServerReady(_) = new_vpn_status {
            self.send_tunnel_command(TunnelCommand::Connect);
        }
    }

    async fn handle_command(&mut self, command: DaemonCommand) {
        match command {
            DaemonCommand::IsAuthenticated(tx) => self.is_authenticated(tx).await,
            DaemonCommand::AccountSignIn(tx, auth_input) => {
                self.on_account_sign_in(tx, auth_input).await
            }
            DaemonCommand::AccountSignOut(tx) => self.on_account_sign_out(tx).await,
            DaemonCommand::ListLocations(tx) => self.on_list_locations(tx).await,
            DaemonCommand::RecentLocations(tx) => self.on_recent_locations(tx).await,
            DaemonCommand::Connect(tx, location) => self.on_connect(tx, location).await,
            DaemonCommand::Disconnect(tx) => self.on_disconnect(tx).await,
            DaemonCommand::GetVpnStatus(tx) => self.on_get_vpn_status(tx).await,
            DaemonCommand::GetNotifications(tx) => self.on_get_notifications(tx).await,
            DaemonCommand::AckNotification(tx, id) => self.on_ack_notification(tx, id).await,
            DaemonCommand::LatestAppVersion(tx) => self.on_latest_app_version(tx).await,
        }
    }

    async fn on_latest_app_version(&self, tx: ResponseTx<String, DaemonError>) {
        let device_handler = self.device_handler.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                device_handler
                    .latest_app_version()
                    .await
                    .map_err(DaemonError::DeviceError),
                "on_latest_app_version",
            );
        });
    }

    async fn on_get_notifications(&self, tx: ResponseTx<Vec<Notification>, DaemonError>) {
        let notifications = self.state.notifications();
        tokio::spawn(async move {
            Self::oneshot_send(tx, Ok(notifications), "on_get_notifications");
        });
    }

    async fn on_ack_notification(&mut self, tx: ResponseTx<(), DaemonError>, id: String) {
        self.state.remove_notification(id);
        tokio::spawn(async move {
            Self::oneshot_send(tx, Ok(()), "on_ack_notification");
        });
    }

    async fn add_notification(&mut self, notification: Notification) {
        // save in current state
        self.state.add_notification(notification.clone());
        // notify event listeners
        self.controller_server_and_event_broadcaster
            .send_notification(notification)
            .await;
    }

    async fn set_vpn_status(&mut self, vpn_status: nymvpn_types::vpn_session::VpnStatus) {
        // save current state
        self.state.set_vpn_status(vpn_status.clone());
        // and notify event listeners,
        {
            // todo: make broadcaster clone-able and send event on spawned task
            self.controller_server_and_event_broadcaster
                .send_vpn_status(vpn_status.into())
                .await;
        }
    }

    async fn on_connect_inner(&mut self, location: Location) -> Result<VpnStatus, DaemonError> {
        if let Some(location) = self.state.vpn_session_in_progress() {
            tracing::warn!("another vpn session in progress: {location}");
            return Err(DaemonError::AnotherVpnSessionInProgress(location));
        }

        let request_id = self
            .vpn_session_storage
            .new_session(location.clone())
            .await?;

        let device_unique_id = self.device_storage.get_device_unique_id().await?;

        let new_session = NewSession {
            request_id,
            device_unique_id,
            location_code: location.code.clone(),
        };

        match self
            .vpn_session_handler
            .new_session(new_session.clone())
            .await
        {
            Ok(accepted) => {
                // update local record
                self.vpn_session_storage
                    .update_on_accepted(accepted)
                    .await?;
                // update state
                self.state.accepted(location.clone());

                Ok(VpnStatus::Accepted(location))
            }
            Err(err) => {
                tracing::error!("cannot connect: {err}");
                // remove local record
                self.vpn_session_storage
                    .delete(new_session.request_id)
                    .await?;
                // add notification about it
                let notification = self
                    .state
                    .add_notification_for_failed_new_session(request_id, location, err);
                self.add_notification(notification).await;
                Ok(VpnStatus::Disconnected)
            }
        }
    }

    async fn on_connect(
        &mut self,
        tx: ResponseTx<VpnStatus, DaemonError>,
        location: nymvpn_types::location::Location,
    ) {
        tracing::info!("Connection requested to {location}");
        let location_storage = self.location_storage.clone();
        let location_to_add = location.clone();
        tokio::spawn(async move {
            if let Err(e) = location_storage.add_recent(location_to_add).await {
                tracing::error!("failed to save recent location: {e}");
            }
        });
        Self::oneshot_send(tx, self.on_connect_inner(location).await, "on_connect");
    }

    async fn tunnel_command_on_disconnect(&self) {
        let current_state = self.state.vpn_status();
        match &current_state {
            VpnStatus::ServerReady(_) | VpnStatus::Connecting(_) | VpnStatus::Connected(_, _) => {
                self.send_tunnel_command(TunnelCommand::Disconnect)
            }
            VpnStatus::Accepted(_)
            | VpnStatus::ServerCreated(_)
            | VpnStatus::ServerRunning(_)
            | VpnStatus::Disconnecting(_)
            | VpnStatus::Disconnected => {
                tracing::info!("Not sending tunnel command to disconnect. State: {current_state}");
            }
        }
    }

    async fn end_session(&mut self, end_reason: String) -> Result<(), DaemonError> {
        // Get session info and mark for deletion in DB
        let session_info = self.vpn_session_storage.end_session().await?;

        match session_info {
            Some(session_info) => {
                let device_unique_id = self.device_storage.get_device_unique_id().await?;

                let end_session = EndSession {
                    request_id: session_info.request_id,
                    device_unique_id,
                    vpn_session_uuid: session_info.vpn_session_id,
                    reason: end_reason,
                };

                // make api call to server to end session,
                // ignore errors as reclaimer should eventually cleanup
                match self
                    .vpn_session_handler
                    .end_session(end_session.clone())
                    .await
                {
                    Ok(ended) => {
                        tracing::info!("vpn session successfully ended on server: {ended}");
                        // on success delete record from DB
                        self.vpn_session_storage
                            .delete(session_info.request_id)
                            .await?;
                    }
                    Err(err) => {
                        tracing::error!("couldn't end session on server {end_session}: {err}");
                    }
                };
            }
            None => {
                tracing::warn!("No existing vpn session found in DB in end_session");
            }
        }

        Ok(())
    }

    async fn client_connected(&self, session_info: SessionInfo) {
        let device_storage = self.device_storage.clone();
        let vpn_session_handler = self.vpn_session_handler.clone();
        tokio::spawn(async move {
            async fn call_client_connected(
                session_info: SessionInfo,
                device_storage: DeviceStorage,
                vpn_session_handler: VpnSessionHandler,
            ) -> Result<(), DaemonError> {
                let device_unique_id = device_storage.get_device_unique_id().await?;

                let client_connected = ClientConnected {
                    request_id: session_info.request_id,
                    device_unique_id,
                    vpn_session_uuid: session_info.vpn_session_id,
                };

                let _ = vpn_session_handler
                    .client_connected(client_connected.clone())
                    .await;
                Ok(())
            }

            if let Err(e) =
                call_client_connected(session_info.clone(), device_storage, vpn_session_handler)
                    .await
            {
                tracing::error!(
                    "couldn't make client connected call on server: {session_info}: {e}"
                );
            }

            Ok::<(), DaemonError>(())
        });
    }

    async fn on_disconnect_inner(&mut self, end_reason: String) -> Result<VpnStatus, DaemonError> {
        self.tunnel_command_on_disconnect().await;
        self.end_session(end_reason).await?;
        let status = self.state.update_state_on_disconnect();
        self.controller_server_and_event_broadcaster
            .send_vpn_status(status.clone())
            .await;
        Ok(status)
    }

    async fn on_disconnect(&mut self, tx: ResponseTx<VpnStatus, DaemonError>) {
        tracing::info!("Disconnect requested");
        Self::oneshot_send(
            tx,
            self.on_disconnect_inner("client requested".into()).await,
            "on_disconnect",
        );
    }

    async fn on_get_vpn_status(&self, tx: ResponseTx<VpnStatus, DaemonError>) {
        let vpn_status = self.state.vpn_status();
        tokio::spawn(async move {
            Self::oneshot_send(tx, Ok(vpn_status), "on_get_vpn_status_response")
        });
    }

    async fn is_authenticated(&self, tx: ResponseTx<bool, DaemonError>) {
        let device_handler = self.device_handler.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                Ok(device_handler.is_authenticated().await),
                "is_authenticated_response",
            )
        });
    }

    async fn on_account_sign_in(
        &self,
        tx: ResponseTx<(), DaemonError>,
        user_creds: UserCredentials,
    ) {
        // if vpn session in progress do not sign in
        if let Some(location) = self.state.vpn_session_in_progress() {
            tracing::warn!("sign in attempt when vpn session in progress: {location}");
            Self::oneshot_send(
                tx,
                Err(DaemonError::InvalidOpVpnSessionInProgress(format!(
                    "cannot sign in when a vpn session is in progress (to city {})",
                    location.city
                ))),
                "on_account_sign_in error",
            );
        } else {
            let device_handler = self.device_handler.clone();
            tokio::spawn(async move {
                Self::oneshot_send(
                    tx,
                    device_handler
                        .sign_in(user_creds)
                        .await
                        .map_err(DaemonError::DeviceError),
                    "on_account_login response",
                )
            });
        }
    }

    async fn on_account_sign_out(&self, tx: ResponseTx<(), DaemonError>) {
        // if vpn session in progress do not sign out
        if let Some(location) = self.state.vpn_session_in_progress() {
            tracing::warn!("sign out attempt when vpn session in progress: {location}");
            Self::oneshot_send(
                tx,
                Err(DaemonError::InvalidOpVpnSessionInProgress(format!(
                    "cannot sign out when a vpn session is in progress (to city {})",
                    location.city
                ))),
                "on_account_sign_out error",
            );
        } else {
            let device_handler = self.device_handler.clone();
            tokio::spawn(async move {
                Self::oneshot_send(
                    tx,
                    device_handler
                        .sign_out()
                        .await
                        .map_err(DaemonError::DeviceError),
                    "on_account_sign_out response",
                )
            });
        }
    }

    async fn on_list_locations(&self, tx: ResponseTx<Vec<Location>, DaemonError>) {
        let vpn_session_handler = self.vpn_session_handler.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                vpn_session_handler
                    .list_locations()
                    .await
                    .map_err(DaemonError::VpnSessionError),
                "on_list_locations response",
            )
        });
    }

    async fn on_recent_locations(&self, tx: ResponseTx<Vec<Location>, DaemonError>) {
        let location_storage = self.location_storage.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                location_storage.recent().await.map_err(DaemonError::DbErr),
                "on_recent_locations response",
            )
        });
    }

    fn send_tunnel_command(&self, command: TunnelCommand) {
        self.tunnel_state_machine_handle
            .command_tx()
            .unbounded_send(command)
            .expect("Tunnel state machine has stopped");
    }

    fn oneshot_send<T>(tx: oneshot::Sender<T>, t: T, msg: &'static str) {
        if tx.send(t).is_err() {
            tracing::warn!("Unable to send {} to the daemon command sender", msg);
        }
    }
}
