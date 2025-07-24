use futures::channel::oneshot;
use nym_credential_verification::ClientBandwidth;
use tokio::sync::mpsc;

use std::net::IpAddr;

use nym_wireguard::peer_controller::{GetClientBandwidthControlResponse, PeerControlRequest};

use crate::error::Error;

pub struct PeerControllerTransceiver {
    request_tx: mpsc::Sender<PeerControlRequest>,
}

impl PeerControllerTransceiver {
    pub fn new(request_tx: mpsc::Sender<PeerControlRequest>) -> Self {
        Self { request_tx }
    }

    async fn get_client_bandwidth(&self, ip: IpAddr) -> Result<ClientBandwidth, Error> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::GetClientBandwidthByIp { ip, response_tx };
        self.request_tx
            .send(msg)
            .await
            .map_err(|_| Error::PeerInteractionStopped)?;

        let GetClientBandwidthControlResponse {
            success,
            client_bandwidth,
        } = response_rx.await.map_err(|_| Error::NoResponse)?;
        if !success {
            return Err(Error::Unsuccessful);
        }
        client_bandwidth.ok_or(Error::NoPeer { ip })
    }

    pub async fn query_bandwidth(&self, ip: IpAddr) -> Result<i64, Error> {
        Ok(self.get_client_bandwidth(ip).await?.available().await)
    }
}

#[cfg(test)]
mod tests {
    use nym_wireguard::CONTROL_CHANNEL_SIZE;

    use super::*;

    #[tokio::test]
    async fn get_bandwidth() {
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let transceiver = PeerControllerTransceiver::new(request_tx);

        tokio::spawn(async move {
            match request_rx.recv().await.unwrap() {
                PeerControlRequest::GetClientBandwidthByIp { ip: _, response_tx } => {
                    response_tx
                        .send(GetClientBandwidthControlResponse {
                            success: true,
                            client_bandwidth: Some(ClientBandwidth::new(Default::default())),
                        })
                        .ok();
                }
                _ => unimplemented!(),
            }
        });

        let bw = transceiver
            .query_bandwidth("10.0.0.42".parse().unwrap())
            .await
            .unwrap();
        assert_eq!(bw, 0);
    }

    #[tokio::test]
    async fn stop_peer() {
        let (request_tx, request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let transceiver = PeerControllerTransceiver::new(request_tx);

        drop(request_rx);
        let err = transceiver
            .query_bandwidth("10.0.0.42".parse().unwrap())
            .await
            .unwrap_err();
        assert_eq!(err, Error::PeerInteractionStopped);
    }

    #[tokio::test]
    async fn unresponsive_peer() {
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let transceiver = PeerControllerTransceiver::new(request_tx);

        tokio::spawn(async move {
            match request_rx.recv().await.unwrap() {
                PeerControlRequest::GetClientBandwidthByIp {
                    ip: _,
                    response_tx: _,
                } => {}
                _ => unimplemented!(),
            }
        });

        let err = transceiver
            .query_bandwidth("10.0.0.42".parse().unwrap())
            .await
            .unwrap_err();
        assert_eq!(err, Error::NoResponse);
    }

    #[tokio::test]
    async fn unsuccessful_query() {
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let transceiver = PeerControllerTransceiver::new(request_tx);

        tokio::spawn(async move {
            match request_rx.recv().await.unwrap() {
                PeerControlRequest::GetClientBandwidthByIp { ip: _, response_tx } => {
                    response_tx
                        .send(GetClientBandwidthControlResponse {
                            success: false,
                            client_bandwidth: None,
                        })
                        .ok();
                }
                _ => unimplemented!(),
            }
        });

        let err = transceiver
            .query_bandwidth("10.0.0.42".parse().unwrap())
            .await
            .unwrap_err();
        assert_eq!(err, Error::Unsuccessful);
    }

    #[tokio::test]
    async fn no_peer() {
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let transceiver = PeerControllerTransceiver::new(request_tx);

        tokio::spawn(async move {
            match request_rx.recv().await.unwrap() {
                PeerControlRequest::GetClientBandwidthByIp { ip: _, response_tx } => {
                    response_tx
                        .send(GetClientBandwidthControlResponse {
                            success: true,
                            client_bandwidth: None,
                        })
                        .ok();
                }
                _ => unimplemented!(),
            }
        });

        let ip = "10.0.0.42".parse().unwrap();
        let err = transceiver.query_bandwidth(ip).await.unwrap_err();
        assert_eq!(err, Error::NoPeer { ip });
    }
}
