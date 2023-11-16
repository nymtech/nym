use std::sync::Arc;

use tokio::{sync::oneshot, task::JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{Request, Response, Status};
use nymvpn_controller::{
    proto::{AccountInfo, Locations, Notifications, SignInRequest, VpnStatus},
    spawn_grpc_server, ControllerError, ControllerService,
};
use nymvpn_migration::DbErr;
use nymvpn_types::notification::Notification;

use crate::{
    daemon::{DaemonCommand, DaemonCommandSender, DaemonError, EventListener},
    device::handler::DeviceHandler,
    shutdown::ShutdownManager,
};

pub struct ControllerServer;

impl ControllerServer {
    pub async fn start(
        daemon_command_sender: DaemonCommandSender,
        shutdown_manager: &ShutdownManager,
        device_handler: DeviceHandler,
    ) -> Result<ControllerServerAndEventBroadcaster, ControllerError> {
        let events_subscribers =
            Arc::<tokio::sync::RwLock<Vec<DaemonEventsListenerSender>>>::default();

        let controller_service = ControllerServiceImpl {
            daemon_command_sender,
            events_subscribers: events_subscribers.clone(),
        };

        let handle = spawn_grpc_server(
            controller_service,
            device_handler,
            shutdown_manager.shutdown_received_future(),
        )
        .await?;

        let controller_server_handle = tokio::spawn(async move {
            if let Err(e) = handle.await {
                tracing::error!("Controller GRPC server error: {e}")
            }
            tracing::info!("Controller server shut down")
        });

        Ok(ControllerServerAndEventBroadcaster {
            events_subscribers,
            controller_server_handle,
        })
    }
}

pub struct ControllerServerAndEventBroadcaster {
    pub events_subscribers: Arc<tokio::sync::RwLock<Vec<DaemonEventsListenerSender>>>,
    pub controller_server_handle: JoinHandle<()>,
}

impl ControllerServerAndEventBroadcaster {
    async fn notify(&self, event: nymvpn_controller::proto::DaemonEvent) {
        let mut subscribers = self.events_subscribers.write().await;

        subscribers.retain(|tx| tx.send(Ok(event.clone())).is_ok());
    }
}

#[async_trait::async_trait]
impl EventListener for ControllerServerAndEventBroadcaster {
    async fn send_vpn_status(&self, status: nymvpn_types::vpn_session::VpnStatus) {
        tracing::debug!("notifying new vpn status");
        self.notify(nymvpn_controller::proto::DaemonEvent {
            event: Some(nymvpn_controller::proto::daemon_event::Event::VpnStatus(
                nymvpn_controller::proto::VpnStatus::from(status),
            )),
        })
        .await
    }

    async fn send_notification(&self, notification: Notification) {
        tracing::debug!("sending new notification");
        self.notify(nymvpn_controller::proto::DaemonEvent {
            event: Some(nymvpn_controller::proto::daemon_event::Event::Notification(
                nymvpn_controller::proto::Notification::from(notification),
            )),
        })
        .await
    }
}

pub struct ControllerServiceImpl {
    daemon_command_sender: DaemonCommandSender,
    events_subscribers: Arc<tokio::sync::RwLock<Vec<DaemonEventsListenerSender>>>,
}

pub type ServiceResult<T> = std::result::Result<Response<T>, Status>;
pub type VpnStatusListenerSender =
    tokio::sync::mpsc::UnboundedSender<Result<nymvpn_controller::proto::VpnStatus, Status>>;
pub type VpnStatusListenerReceiver =
    UnboundedReceiverStream<Result<nymvpn_controller::proto::VpnStatus, Status>>;

pub type DaemonEventsListenerSender =
    tokio::sync::mpsc::UnboundedSender<Result<nymvpn_controller::proto::DaemonEvent, Status>>;

pub type DaemonEventsListenerReceiver =
    UnboundedReceiverStream<Result<nymvpn_controller::proto::DaemonEvent, Status>>;

#[tonic::async_trait]
impl ControllerService for ControllerServiceImpl {
    type WatchEventsStream = DaemonEventsListenerReceiver;

    /// Locations served
    async fn get_locations(&self, _: Request<()>) -> ServiceResult<Locations> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::ListLocations(tx))?;
        self.wait_for_result(rx)
            .await?
            .map(|locations| Response::new(locations.into()))
            .map_err(map_daemon_error)
    }

    async fn recent_locations(&self, _: Request<()>) -> ServiceResult<Locations> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::RecentLocations(tx))?;
        self.wait_for_result(rx)
            .await?
            .map(|locations| Response::new(locations.into()))
            .map_err(map_daemon_error)
    }

    /// Account
    async fn is_authenticated(&self, _req: Request<()>) -> ServiceResult<bool> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::IsAuthenticated(tx))?;
        self.wait_for_result(rx)
            .await?
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    async fn account_sign_in(&self, req: Request<SignInRequest>) -> ServiceResult<()> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::AccountSignIn(tx, req.into_inner().into()))?;
        self.wait_for_result(rx)
            .await?
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    async fn account_sign_out(&self, _: Request<()>) -> ServiceResult<()> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::AccountSignOut(tx))?;
        self.wait_for_result(rx)
            .await?
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    async fn get_account_info(&self, _: Request<()>) -> ServiceResult<AccountInfo> {
        todo!()
    }

    /// Control VPN
    async fn connect_vpn(
        &self,
        req: Request<nymvpn_controller::proto::Location>,
    ) -> ServiceResult<VpnStatus> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::Connect(tx, req.into_inner().into()))?;

        self.wait_for_result(rx)
            .await?
            .map(nymvpn_controller::proto::VpnStatus::from)
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    async fn disconnect_vpn(&self, _: Request<()>) -> ServiceResult<VpnStatus> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::Disconnect(tx))?;
        self.wait_for_result(rx)
            .await?
            .map(nymvpn_controller::proto::VpnStatus::from)
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    async fn get_vpn_status(&self, _: Request<()>) -> ServiceResult<VpnStatus> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::GetVpnStatus(tx))?;

        self.wait_for_result(rx)
            .await?
            .map(nymvpn_controller::proto::VpnStatus::from)
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    /// Notifications
    async fn get_notifications(&self, _: Request<()>) -> ServiceResult<Notifications> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::GetNotifications(tx))?;

        self.wait_for_result(rx)
            .await?
            .map(nymvpn_controller::proto::Notifications::from)
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    async fn ack_notification(&self, id: Request<String>) -> ServiceResult<()> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::AckNotification(tx, id.into_inner()))?;

        self.wait_for_result(rx)
            .await?
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    // Versions and Updates
    async fn latest_app_version(&self, _: Request<()>) -> ServiceResult<String> {
        let (tx, rx) = oneshot::channel();
        self.send_command_to_daemon(DaemonCommand::LatestAppVersion(tx))?;

        self.wait_for_result(rx)
            .await?
            .map(Response::new)
            .map_err(map_daemon_error)
    }

    /// Event stream
    async fn watch_events(&self, _: Request<()>) -> ServiceResult<Self::WatchEventsStream> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let mut subscribers = self.events_subscribers.write().await;
        subscribers.push(tx);
        Ok(Response::new(UnboundedReceiverStream::new(rx)))
    }
}

impl ControllerServiceImpl {
    fn send_command_to_daemon(&self, command: DaemonCommand) -> Result<(), Status> {
        self.daemon_command_sender
            .send(command)
            .map_err(|_| Status::internal("the daemon channel receiver has been dropped"))
    }

    async fn wait_for_result<T>(&self, rx: tokio::sync::oneshot::Receiver<T>) -> Result<T, Status> {
        rx.await.map_err(|_| Status::internal("sender was dropped"))
    }
}

fn map_db_error(_db_err: DbErr) -> Status {
    Status::internal("daemon is unable to manage its database")
}

pub const SERVER_UNAVAILABLE_PLEASE_TRY_AGAIN_LATER: &str =
    "server is unavailable, please try again later";
pub const VPN_SESSION_SERVICE_UNAVAILABLE: &str =
    "daemon is partially up: vpn session service unavailable";

fn map_daemon_error(error: DaemonError) -> Status {
    tracing::error!("{:?}", error);
    match error {
        DaemonError::DaemonUnavailable => Status::internal("daemon is unavailable"),
        DaemonError::AnotherVpnSessionInProgress(location) => Status::failed_precondition(format!(
            "cannot start a new vpn session when another is in progress (to city {})",
            location.city
        )),
        DaemonError::InvalidOpVpnSessionInProgress(message) => Status::failed_precondition(message),
        DaemonError::DbErr(db_err) => map_db_error(db_err),
        DaemonError::DeviceError(device_error) => match device_error {
            crate::device::DeviceError::DeviceServiceUnavailable => {
                Status::internal("daemon is partially up: device service unavailable")
            }
            crate::device::DeviceError::Server(status) => status,
            crate::device::DeviceError::Connection(_) => {
                Status::unavailable(SERVER_UNAVAILABLE_PLEASE_TRY_AGAIN_LATER)
            }
            crate::device::DeviceError::DbErr(db_err) => map_db_error(db_err),
            crate::device::DeviceError::InitError(_) => {
                Status::internal("failed to initialize device")
            }
        },
        DaemonError::VpnSessionError(vpn_session_error) => match vpn_session_error {
            crate::vpn_session::handler::VpnSessionError::VpnSessionServiceDown => {
                Status::internal(VPN_SESSION_SERVICE_UNAVAILABLE)
            }
            crate::vpn_session::handler::VpnSessionError::Connection(_) => {
                Status::unavailable(SERVER_UNAVAILABLE_PLEASE_TRY_AGAIN_LATER)
            }
            crate::vpn_session::handler::VpnSessionError::Server(status) => status,
        },
    }
}
