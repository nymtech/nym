use std::time::Duration;

use futures::future::abortable;
use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;
use tokio_stream::StreamExt;
use nymvpn_types::notification::Notification;

#[derive(Debug)]
pub struct EventForwarderHandler {
    _shutdown_tx: oneshot::Sender<()>,
}

impl EventForwarderHandler {
    pub async fn start(app_handle: AppHandle) -> Self {
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let event_forwarder = EventForwarder::new(app_handle, shutdown_rx);
        tauri::async_runtime::spawn(async move {
            event_forwarder.run().await;
        });
        Self { _shutdown_tx }
    }
}

struct EventForwarder {
    app_handle: AppHandle,
    shutdown_rx: oneshot::Receiver<()>,
}

impl EventForwarder {
    fn new(app_handle: AppHandle, shutdown_rx: oneshot::Receiver<()>) -> Self {
        Self {
            app_handle,
            shutdown_rx,
        }
    }

    async fn event_watch_loop(app_handle: AppHandle) {
        loop {
            match nymvpn_controller::new_grpc_client().await {
                Ok(mut client) => {
                    log::info!("listening to daemon events ...");
                    match client.watch_events(()).await {
                        Ok(stream) => {
                            let mut stream = stream.into_inner();
                            while let Some(event) = stream.next().await {
                                if let Ok(event) = event {
                                    if let Some(event) = event.event {
                                        match event {
                                            nymvpn_controller::proto::daemon_event::Event::VpnStatus(vpn_status) => {
                                                let vpn_status: nymvpn_types::vpn_session::VpnStatus = vpn_status.into();
                                                let _ = app_handle.emit_all("vpn_status", vpn_status);
                                            },
                                            nymvpn_controller::proto::daemon_event::Event::Notification(notification) => {
                                                if let Ok(notification) = Notification::try_from(notification) {
                                                    let _ = app_handle.emit_all("notification", notification);
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        Err(status) => {
                            log::error!("cannot receive events: {}", status.message());
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    };
                }
                Err(err) => {
                    log::error!("daemon is offline: {err}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn run(self) {
        let (event_watch_future, abort_handle) = abortable(Self::event_watch_loop(self.app_handle));

        tauri::async_runtime::spawn(event_watch_future);

        let _ = self.shutdown_rx.await;

        abort_handle.abort();

        log::info!("event forwarder shut down");
    }
}
