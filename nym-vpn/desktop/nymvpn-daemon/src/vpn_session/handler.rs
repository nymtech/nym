use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use nymvpn_server::{auth::TokenProvider, ServerApi};
use nymvpn_types::{
    location::Location,
    nymvpn_server::{
        Accepted, ClientConnected, EndSession, Ended, NewSession, VpnSessionStatusRequest,
    },
};
use uuid::Uuid;

use crate::{daemon::DaemonEventSender, AckTx, ResponseTx};

use super::watcher::WatcherFactory;

pub enum VpnSessionCommand {
    NewSession(ResponseTx<Accepted, VpnSessionError>, NewSession),
    EndSession(ResponseTx<Ended, VpnSessionError>, EndSession),
    ClientConnected(ResponseTx<(), VpnSessionError>, ClientConnected),
    ListLocations(ResponseTx<Vec<Location>, VpnSessionError>),
    Shutdown(AckTx),
}

#[derive(Debug, thiserror::Error)]
pub enum VpnSessionError {
    #[error("vpn session service is unavailable")]
    VpnSessionServiceDown,
    #[error("error connecting to server: {0}")]
    Connection(#[from] tonic::transport::Error),
    #[error("server error: {0}")]
    Server(#[from] tonic::Status),
}

#[derive(Debug, Clone)]
pub struct VpnSessionHandler {
    tx: mpsc::UnboundedSender<VpnSessionCommand>,
}

impl VpnSessionHandler {
    pub async fn start<P: TokenProvider + 'static>(
        daemon_tx: DaemonEventSender,
        token_provider: P,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let vpn_session_service = VpnSessionService::new(rx, daemon_tx, token_provider);

        tokio::spawn(async move { vpn_session_service.run().await });

        Self { tx }
    }

    pub async fn shutdown(&self) -> Result<(), VpnSessionError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(VpnSessionCommand::Shutdown(tx))
            .map_err(|_e| VpnSessionError::VpnSessionServiceDown)?;
        rx.await
            .map_err(|_e| VpnSessionError::VpnSessionServiceDown)?;
        Ok(())
    }

    pub async fn new_session(
        &self,
        new_session: nymvpn_types::nymvpn_server::NewSession,
    ) -> Result<Accepted, VpnSessionError> {
        self.send_command(|tx| VpnSessionCommand::NewSession(tx, new_session))
            .await
    }

    pub async fn end_session(
        &self,
        end_session: nymvpn_types::nymvpn_server::EndSession,
    ) -> Result<Ended, VpnSessionError> {
        self.send_command(|tx| VpnSessionCommand::EndSession(tx, end_session))
            .await
    }

    pub async fn client_connected(
        &self,
        client_connected: ClientConnected,
    ) -> Result<(), VpnSessionError> {
        self.send_command(|tx| VpnSessionCommand::ClientConnected(tx, client_connected))
            .await
    }

    pub async fn list_locations(&self) -> Result<Vec<Location>, VpnSessionError> {
        self.send_command(|tx| VpnSessionCommand::ListLocations(tx))
            .await
    }

    pub async fn send_command<T>(
        &self,
        make_cmd: impl FnOnce(ResponseTx<T, VpnSessionError>) -> VpnSessionCommand,
    ) -> Result<T, VpnSessionError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(make_cmd(tx))
            .map_err(|_| VpnSessionError::VpnSessionServiceDown)?;
        rx.await
            .map_err(|_| VpnSessionError::VpnSessionServiceDown)?
    }
}

pub struct VpnSessionService<P: TokenProvider> {
    daemon_tx: DaemonEventSender,
    receiver: mpsc::UnboundedReceiver<VpnSessionCommand>,
    token_provider: P,
    shutdown_tx: Option<AckTx>,
    watcher_shutdown_txs: HashMap<Uuid, oneshot::Sender<()>>,
}

impl<P: TokenProvider + 'static> VpnSessionService<P> {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<VpnSessionCommand>,
        daemon_tx: DaemonEventSender,
        token_provider: P,
    ) -> Self {
        Self {
            daemon_tx,
            receiver,
            token_provider,
            shutdown_tx: None,
            watcher_shutdown_txs: Default::default(),
        }
    }

    pub async fn run(mut self) {
        while let Some(command) = self.receiver.recv().await {
            self.handle_command(command).await;
            if self.shutdown_tx.is_some() {
                break;
            }
        }

        if self.shutdown_tx.is_some() {
            // stop all watchers
            let _ = self
                .watcher_shutdown_txs
                .into_iter()
                .map(|(_request_id, tx)| tx.send(()));

            // ack shutdown
            if let Err(_) = self.shutdown_tx.unwrap().send(()) {
                tracing::error!("failed to ack vpn session service shutdown");
            };
        }

        tracing::info!("vpn session service stopped");
    }

    async fn handle_command(&mut self, command: VpnSessionCommand) {
        match command {
            VpnSessionCommand::NewSession(tx, new_session) => {
                self.on_new_session(tx, new_session).await
            }
            VpnSessionCommand::EndSession(tx, end_session) => {
                self.on_end_session(tx, end_session).await
            }
            VpnSessionCommand::ClientConnected(tx, client_connected) => {
                self.on_client_connected(tx, client_connected).await
            }
            VpnSessionCommand::Shutdown(ack_tx) => self.shutdown_tx = Some(ack_tx),
            VpnSessionCommand::ListLocations(tx) => self.on_list_locations(tx).await,
        }
    }

    async fn on_new_session_inner(
        token_provider: impl TokenProvider + 'static,
        new_session: NewSession,
    ) -> Result<Accepted, VpnSessionError> {
        let mut nymvpn_service = ServerApi::new(token_provider).await?;
        Ok(nymvpn_service.new_session(new_session).await?)
    }

    async fn on_new_session(
        &mut self,
        tx: ResponseTx<Accepted, VpnSessionError>,
        new_session: NewSession,
    ) {
        let token_provider = self.token_provider.clone();
        let accepted = Self::on_new_session_inner(token_provider, new_session.clone()).await;

        // if accepted start a watcher
        if let Ok(accepted) = &accepted {
            let vpn_session_status_request = VpnSessionStatusRequest {
                request_id: new_session.request_id,
                vpn_session_uuid: accepted.vpn_session_uuid,
                device_unique_id: new_session.device_unique_id,
            };
            self.start_watcher(vpn_session_status_request).await;
        }

        Self::oneshot_send(tx, accepted, "on_list_locations");
    }

    async fn on_end_session_inner(
        token_provider: impl TokenProvider + 'static,
        end_session: EndSession,
    ) -> Result<Ended, VpnSessionError> {
        let mut nymvpn_service = ServerApi::new(token_provider).await?;
        Ok(nymvpn_service.end_session(end_session).await?)
    }

    async fn start_watcher(&mut self, vpn_session_status_request: VpnSessionStatusRequest) {
        let daemon_tx = self.daemon_tx.clone();
        let token_provider = self.token_provider.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.watcher_shutdown_txs
            .insert(vpn_session_status_request.request_id, shutdown_tx);

        tokio::spawn(async move {
            WatcherFactory::start(
                vpn_session_status_request,
                daemon_tx,
                shutdown_rx,
                token_provider,
            )
            .await;
        });
    }

    fn stop_watcher(&mut self, request_id: &Uuid) {
        let _ = self
            .watcher_shutdown_txs
            .remove_entry(request_id)
            .map(|(_request_id, tx)| tx.send(()));
    }

    async fn on_end_session(
        &mut self,
        tx: ResponseTx<Ended, VpnSessionError>,
        end_session: EndSession,
    ) {
        // stop watcher if any
        self.stop_watcher(&end_session.request_id);

        let token_provider = self.token_provider.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                Self::on_end_session_inner(token_provider, end_session).await,
                "on_end_session",
            )
        });
    }

    async fn on_client_connected_inner(
        token_provider: impl TokenProvider + 'static,
        client_connected: ClientConnected,
    ) -> Result<(), VpnSessionError> {
        let mut nymvpn_service = ServerApi::new(token_provider).await?;
        Ok(nymvpn_service.client_connected(client_connected).await?)
    }

    async fn on_client_connected(
        &mut self,
        tx: ResponseTx<(), VpnSessionError>,
        client_connected: ClientConnected,
    ) {
        // stop watcher if any
        self.stop_watcher(&client_connected.request_id);

        let token_provider = self.token_provider.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                Self::on_client_connected_inner(token_provider, client_connected).await,
                "on_client_connected",
            )
        });
    }

    async fn on_list_locations_inner(
        token_provider: impl TokenProvider + 'static,
    ) -> Result<Vec<Location>, VpnSessionError> {
        let mut nymvpn_service = ServerApi::new(token_provider).await?;
        let locations = nymvpn_service.list_locations().await?;
        Ok(locations)
    }

    async fn on_list_locations(&self, tx: ResponseTx<Vec<Location>, VpnSessionError>) {
        let token_provider = self.token_provider.clone();
        tokio::spawn(async move {
            Self::oneshot_send(
                tx,
                Self::on_list_locations_inner(token_provider).await,
                "on_list_locations",
            )
        });
    }

    fn oneshot_send<T>(tx: oneshot::Sender<T>, t: T, msg: &'static str) {
        if tx.send(t).is_err() {
            tracing::warn!("Failed to respond from VpnSessionService {}", msg);
        }
    }
}
