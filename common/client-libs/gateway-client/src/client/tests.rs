// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Ticket accounting after a failed bandwidth claim: a ticket that may have reached the gateway is never reused.
//! The fake gateway binds an ephemeral loopback port per test, so tests share nothing.

use super::*;
use crate::packet_router::{AcknowledgementReceiver, MixnetMessageReceiver};
use futures::{SinkExt, StreamExt};
use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_bandwidth_controller::mock::MockBandwidthController;
use nym_bandwidth_controller::{
    BandwidthTicketProvider, PreparedCredential, PreparedCredentialMetadata,
};
use nym_gateway_requests::SimpleGatewayRequestsError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::TcpListener;

/// Hands out the mock credential and counts revert attempts.
struct RecordingTicketProvider {
    inner: MockBandwidthController,
    reverts: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl BandwidthTicketProvider for RecordingTicketProvider {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        self.inner
            .get_ecash_ticket(ticket_type, gateway_id, tickets_to_spend, spend_time)
            .await
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        Ok(None)
    }

    async fn attempt_revert_spending(
        &self,
        _: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        self.reverts.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }

    async fn close(&self) {}
}

/// What the fake gateway does after reading the client's claim.
enum GatewayBehaviour {
    /// Drop the connection without answering: the claim left the client but no verdict came back.
    Vanish,
    /// Answer with the typed replay rejection.
    RejectAsReplay,
}

/// Bind a one-shot websocket gateway on an ephemeral loopback port and return its URL.
async fn spawn_fake_gateway(behaviour: GatewayBehaviour) -> Url {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        // the encrypted credential claim
        let _claim = ws.next().await;
        match behaviour {
            GatewayBehaviour::Vanish => drop(ws),
            GatewayBehaviour::RejectAsReplay => {
                let response = ServerResponse::TypedError {
                    error: SimpleGatewayRequestsError::TicketReplay,
                };
                ws.send(response.into()).await.unwrap();
            }
        }
    });
    Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap()
}

struct Harness {
    client: GatewayClient,
    reverts: Arc<AtomicUsize>,
    // kept alive so the packet router's channels stay open
    _ack_rx: AcknowledgementReceiver,
    _mix_rx: MixnetMessageReceiver,
}

/// A client in the post-authentication state `claim_bandwidth` requires, without running the handshake.
fn authenticated_client(gateway_url: Url) -> Harness {
    let mut rng = rand::rng();
    let gateway_identity = *ed25519::KeyPair::new(&mut rng).public_key();
    let local_identity = Arc::new(ed25519::KeyPair::new(&mut rng));
    let shared_key = SharedSymmetricKey::try_from_bytes(&[7u8; 32]).unwrap();
    let (ack_tx, ack_rx) = futures::channel::mpsc::unbounded();
    let (mix_tx, mix_rx) = futures::channel::mpsc::unbounded();
    let shutdown = ShutdownToken::default();
    let reverts = Arc::new(AtomicUsize::new(0));
    let provider = RecordingTicketProvider {
        inner: MockBandwidthController::default(),
        reverts: reverts.clone(),
    };

    let mut cfg = GatewayClientConfig::new_default().with_response_timeout(Duration::from_secs(5));
    cfg.bandwidth.require_tickets = true;

    let mut client = GatewayClient::new(
        cfg,
        GatewayConfig::new(
            gateway_identity,
            GatewayListeners {
                primary: gateway_url,
                fallback: None,
            },
        ),
        local_identity,
        Some(Arc::new(shared_key)),
        PacketRouter::new(ack_tx, mix_tx, shutdown.clone()),
        Box::new(provider),
        ClientStatsSender::new(None, shutdown.clone()),
        #[cfg(unix)]
        None,
        shutdown,
    );
    client.authenticated = true;
    client.negotiated_protocol = Some(GatewayProtocolVersion::CURRENT);

    Harness {
        client,
        reverts,
        _ack_rx: ack_rx,
        _mix_rx: mix_rx,
    }
}

/// No connection was ever established, so the claim cannot have reached a gateway: the ticket goes back.
#[tokio::test]
async fn claim_failing_before_transmission_reverts_the_ticket() {
    let mut h = authenticated_client(Url::parse("ws://127.0.0.1:1").unwrap());
    let err = h.client.claim_bandwidth().await.unwrap_err();
    assert!(matches!(err, GatewayClientError::ConnectionNotEstablished));
    assert_eq!(h.reverts.load(Ordering::SeqCst), 1);
}

/// The gateway read the claim and vanished: it may have recorded the serial, so the ticket stays spent.
#[tokio::test]
async fn claim_failing_after_transmission_keeps_the_ticket_spent() {
    let url = spawn_fake_gateway(GatewayBehaviour::Vanish).await;
    let mut h = authenticated_client(url);
    h.client.establish_connection().await.unwrap();
    let _err = h.client.claim_bandwidth().await.unwrap_err();
    assert_eq!(h.reverts.load(Ordering::SeqCst), 0);
}

/// A replay verdict is a decision about a ticket the gateway has already seen.
#[tokio::test]
async fn replay_rejection_is_typed_and_keeps_the_ticket_spent() {
    let url = spawn_fake_gateway(GatewayBehaviour::RejectAsReplay).await;
    let mut h = authenticated_client(url);
    h.client.establish_connection().await.unwrap();
    let err = h.client.claim_bandwidth().await.unwrap_err();
    assert!(err.is_ticket_replay());
    assert_eq!(h.reverts.load(Ordering::SeqCst), 0);
}
