use tokio::time::{self, Duration};
use nymvpn_types::nymvpn_server::EndSession;

use crate::{device::storage::DeviceStorage, shutdown::Shutdown};

use super::{handler::VpnSessionHandler, storage::VpnSessionStorage};

pub struct ReclaimerCreator;

impl ReclaimerCreator {
    pub async fn start(
        session_storage: VpnSessionStorage,
        device_storage: DeviceStorage,
        vpn_session_handler: VpnSessionHandler,
        shutdown: Shutdown,
    ) {
        let reclaimer = Reclaimer::new(
            session_storage,
            device_storage,
            vpn_session_handler,
            shutdown,
        );

        tokio::spawn(async move {
            reclaimer.run().await;
        });
    }
}

pub struct Reclaimer {
    session_storage: VpnSessionStorage,
    device_storage: DeviceStorage,
    vpn_session_handler: VpnSessionHandler,
    shutdown: Shutdown,
}

impl Reclaimer {
    pub fn new(
        session_storage: VpnSessionStorage,
        device_storage: DeviceStorage,
        vpn_session_handler: VpnSessionHandler,
        shutdown: Shutdown,
    ) -> Self {
        Self {
            session_storage,
            device_storage,
            vpn_session_handler,
            shutdown,
        }
    }

    async fn reclaim(&mut self) {
        // Delay to avoid race when the session was successfully ended just now.
        time::sleep(Duration::from_millis(300)).await;
        if let Ok(sessions) = self.session_storage.to_reclaim().await {
            if let Ok(Some(device_details)) = self.device_storage.get_device().await {
                for session in sessions {
                    let end_session = EndSession {
                        request_id: session.request_id,
                        device_unique_id: device_details.unique_id,
                        vpn_session_uuid: session.vpn_session_id,
                        reason: "reclaimed".into(),
                    };

                    match self.vpn_session_handler.end_session(end_session).await {
                        Ok(_) => {
                            tracing::info!("Reclaimed: {session}");
                            let _ = self.session_storage.delete(session.request_id).await;
                        }
                        Err(e) => {
                            match e {
                            crate::vpn_session::handler::VpnSessionError::VpnSessionServiceDown
                            | crate::vpn_session::handler::VpnSessionError::Connection(_) => {
                                // no-op reclaimer would re-run again
                            }
                            crate::vpn_session::handler::VpnSessionError::Server(status) => {
                                // did the best we can, delete it from local storage
                                // this could happen when device is signed out in middle of reclaiming
                                // signing out ends all sessions on server so its good to delete locally.
                                tracing::info!(
                                    "Did best to reclaim session: {session}; server status: {status}"
                                );
                                let _ = self.session_storage.delete(session.request_id).await;
                            }
                        }
                        }
                    }
                }
            }
        };
    }

    pub async fn run(mut self) {
        let mut duration = Duration::from_secs(1);
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                _ = time::sleep(duration) => {
                    self.reclaim().await;
                    duration = Duration::from_secs(60);
                }
                _ = self.shutdown.recv() => {
                    tracing::info!("Reclaimer shutting down");
                }
            }
        }
    }
}
