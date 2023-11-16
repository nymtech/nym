use tokio::sync::oneshot;
use tokio::time::{interval, Duration};
use nymvpn_server::auth::TokenProvider;
use nymvpn_server::ServerApi;
use nymvpn_types::nymvpn_server::{VpnSessionStatus, VpnSessionStatusRequest};

use crate::daemon::{DaemonEvent, DaemonEventSender};

pub struct Watcher<P: TokenProvider + 'static> {
    daemon_tx: DaemonEventSender,
    vpn_session_status_request: VpnSessionStatusRequest,
    token_provider: P,
    last_watch: Option<VpnSessionStatus>,
}

impl<P: TokenProvider + 'static> Watcher<P> {
    pub fn new(
        vpn_session_status_request: VpnSessionStatusRequest,
        daemon_tx: DaemonEventSender,
        token_provider: P,
    ) -> Self {
        Watcher {
            vpn_session_status_request,
            daemon_tx,
            token_provider,
            last_watch: None,
        }
    }

    // return true if watch output reached "final" state
    async fn watch_ended(&mut self) -> bool {
        let token_provider = self.token_provider.clone();
        let vpn_session_status_request = self.vpn_session_status_request.clone();

        let nymvpn_service = ServerApi::new(token_provider).await;
        match nymvpn_service {
            Ok(mut nymvpn_service) => {
                match nymvpn_service
                    .get_status(vpn_session_status_request.clone())
                    .await
                {
                    Ok(vpn_session_status) => {
                        let last_vpn_session_status = std::mem::replace(
                            &mut self.last_watch,
                            Some(vpn_session_status.clone()),
                        );

                        if let Some(last_vpn_session_status) = last_vpn_session_status {
                            if last_vpn_session_status == vpn_session_status {
                                return false;
                            }
                        }

                        if let Err(e) = self
                            .daemon_tx
                            .send(DaemonEvent::VpnSessionStatus(vpn_session_status.clone()))
                        {
                            tracing::error!("Failed to notify daemon from watcher; ending watch for {vpn_session_status_request}: {e}");
                            return true;
                        }

                        match vpn_session_status {
                            nymvpn_types::nymvpn_server::VpnSessionStatus::Failed(_)
                            | nymvpn_types::nymvpn_server::VpnSessionStatus::ServerReady(_)
                            | nymvpn_types::nymvpn_server::VpnSessionStatus::ClientConnected(_)
                            | nymvpn_types::nymvpn_server::VpnSessionStatus::Ended(_) => {
                                tracing::info!("watcher end state received: {vpn_session_status}");
                                return true;
                            }
                            nymvpn_types::nymvpn_server::VpnSessionStatus::ServerCreated(_)
                            | nymvpn_types::nymvpn_server::VpnSessionStatus::ServerRunning(_)
                            | nymvpn_types::nymvpn_server::VpnSessionStatus::Accepted(_) => {}
                        };
                    }
                    Err(err) => {
                        // todo: this could be transient error? so we don't end the watch here
                        tracing::error!(
                            "watch received error from server for {vpn_session_status_request}: {}",
                            err.message()
                        )
                    }
                }
            }
            Err(err) => {
                // transient error: don't end the watch here
                tracing::error!("failed to connect to nymvpn service from watcher for {vpn_session_status_request}: {err}");
            }
        }
        false
    }

    pub async fn run(mut self, mut shutdown_rx: oneshot::Receiver<()>) {
        let mut interval = interval(Duration::from_millis(1000));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.watch_ended().await {
                        break;
                    }
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("watcher received shutdown");
                    break;
                }
            }
        }
        tracing::info!("watcher stopped");
    }
}

pub struct WatcherFactory;

impl WatcherFactory {
    pub async fn start(
        vpn_session_status_request: VpnSessionStatusRequest,
        daemon_tx: DaemonEventSender,
        shutdown_rx: oneshot::Receiver<()>,
        token_provider: impl TokenProvider + 'static,
    ) {
        let watcher = Watcher::new(vpn_session_status_request, daemon_tx, token_provider);
        tokio::spawn(async move { watcher.run(shutdown_rx).await });
    }
}
