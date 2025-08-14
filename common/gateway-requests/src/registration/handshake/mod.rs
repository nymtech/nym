// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::error::HandshakeError;
use crate::registration::handshake::state::State;
use crate::{GatewayProtocolVersion, SharedGatewayKey};
use futures::future::BoxFuture;
use futures::{Sink, Stream};
use nym_crypto::asymmetric::ed25519;
use rand::{CryptoRng, RngCore};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tungstenite::{Error as WsError, Message as WsMessage};

#[cfg(not(target_arch = "wasm32"))]
use nym_task::ShutdownToken;

pub(crate) type WsItem = Result<WsMessage, WsError>;

mod client;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
mod gateway;
mod messages;
mod state;

// realistically even 32bit would have sufficed, so 128 is definitely enough
pub const KDF_SALT_LENGTH: usize = 16;

// Note: the handshake is built on top of WebSocket, but in principle it shouldn't be too difficult
// to remove that restriction, by just changing Sink<WsMessage> and Stream<Item = WsMessage> into
// AsyncWrite and AsyncRead and slightly adjusting the implementation. But right now
// we do not need to worry about that.

pub struct GatewayHandshake<'a> {
    handshake_future: BoxFuture<'a, Result<HandshakeResult, HandshakeError>>,
}

impl Future for GatewayHandshake<'_> {
    type Output = Result<HandshakeResult, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}

#[derive(Debug, PartialEq)]
pub struct HandshakeResult {
    pub negotiated_protocol: GatewayProtocolVersion,
    pub derived_key: SharedGatewayKey,
}

pub fn client_handshake<'a, S, R>(
    rng: &'a mut R,
    ws_stream: &'a mut S,
    identity: &'a ed25519::KeyPair,
    gateway_pubkey: ed25519::PublicKey,
    gateway_protocol: Option<GatewayProtocolVersion>,
    #[cfg(not(target_arch = "wasm32"))] shutdown_token: ShutdownToken,
) -> GatewayHandshake<'a>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
    R: CryptoRng + RngCore + Send,
{
    let state = State::new(
        rng,
        ws_stream,
        identity,
        Some(gateway_pubkey),
        gateway_protocol,
        #[cfg(not(target_arch = "wasm32"))]
        shutdown_token,
    );

    GatewayHandshake {
        handshake_future: Box::pin(state.perform_client_handshake()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_handshake<'a, S, R>(
    rng: &'a mut R,
    ws_stream: &'a mut S,
    identity: &'a ed25519::KeyPair,
    received_init_payload: Vec<u8>,
    requested_client_protocol: Option<GatewayProtocolVersion>,
    shutdown_token: ShutdownToken,
) -> GatewayHandshake<'a>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
    R: CryptoRng + RngCore + Send,
{
    let state = State::new(
        rng,
        ws_stream,
        identity,
        None,
        requested_client_protocol,
        shutdown_token,
    );
    GatewayHandshake {
        handshake_future: Box::pin(state.perform_gateway_handshake(received_init_payload)),
    }
}

/*

Messages exchanged:

CLIENT -> GATEWAY:
CLIENT_ID_KEY || G^x

GATEWAY -> CLIENT
G^y || AES(k, SIG(PRIV_G, G^y || G^x))

CLIENT -> GATEWAY
AES(k, SIG(PRIV_C, G^x || G^y))

GATEWAY -> CLIENT
DONE(status)

*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClientControlRequest, CURRENT_PROTOCOL_VERSION, INITIAL_PROTOCOL_VERSION};
    use anyhow::{bail, Context};
    use futures::StreamExt;
    use nym_test_utils::helpers::u64_seeded_rng;
    use nym_test_utils::mocks::stream_sink::mock_streams;
    use nym_test_utils::traits::{Leak, Timeboxed, TimeboxedSpawnable};
    use tokio::join;
    use tungstenite::Message;

    trait ClientControlRequestExt {
        async fn get_handshake_init_data(&mut self) -> anyhow::Result<Vec<u8>> {
            let ClientControlRequest::RegisterHandshakeInitRequest {
                protocol_version: _,
                data,
            } = self.get_control_request().await?
            else {
                bail!("unexpected ClientControlRequest")
            };
            Ok(data)
        }
        async fn get_control_request(&mut self) -> anyhow::Result<ClientControlRequest>;
    }

    impl<T> ClientControlRequestExt for T
    where
        T: Stream<Item = WsItem> + Unpin,
    {
        async fn get_control_request(&mut self) -> anyhow::Result<ClientControlRequest> {
            let msg = self
                .next()
                .timeboxed()
                .await
                .context("timeout")?
                .context("no message!")??
                .into_text()?
                .parse::<ClientControlRequest>()?;
            Ok(msg)
        }
    }

    struct Party<R: 'static, S: 'static> {
        rng: &'static mut R,
        keys: &'static mut ed25519::KeyPair,
        socket: &'static mut S,
    }

    fn setup() -> (
        Party<
            impl CryptoRng + RngCore + Send,
            impl Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
        >,
        Party<
            impl CryptoRng + RngCore + Send,
            impl Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
        >,
    ) {
        // solve the lifetime issue by just leaking the contents of the boxes
        // which is perfectly fine in test
        let client_rng = u64_seeded_rng(42).leak();
        let gateway_rng = u64_seeded_rng(69).leak();

        let client_keys = ed25519::KeyPair::new(client_rng).leak();
        let gateway_keys = ed25519::KeyPair::new(gateway_rng).leak();

        let (client_ws, gateway_ws) = mock_streams::<Message>();

        // we need streams that return Result<Message, WsError>
        let client_ws = client_ws.map(Ok);
        let gateway_ws = gateway_ws.map(Ok);

        let client_ws = client_ws.leak();
        let gateway_ws = gateway_ws.leak();

        (
            Party {
                rng: client_rng,
                keys: client_keys,
                socket: client_ws,
            },
            Party {
                rng: gateway_rng,
                keys: gateway_keys,
                socket: gateway_ws,
            },
        )
    }

    #[tokio::test]
    async fn basic_handshake() -> anyhow::Result<()> {
        let (client, gateway) = setup();

        let handshake_client = client_handshake(
            client.rng,
            client.socket,
            client.keys,
            *gateway.keys.public_key(),
            Some(CURRENT_PROTOCOL_VERSION),
            ShutdownToken::default(),
        );

        let client_fut = handshake_client.spawn_timeboxed();

        // we need to receive the first message so that it could be propagated to the gateway side of the handshake
        let init_msg = gateway.socket.get_handshake_init_data().await?;

        let handshake_gateway = gateway_handshake(
            gateway.rng,
            gateway.socket,
            gateway.keys,
            init_msg,
            Some(CURRENT_PROTOCOL_VERSION),
            ShutdownToken::default(),
        );

        let gateway_fut = handshake_gateway.spawn_timeboxed();
        let (client, gateway) = join!(client_fut, gateway_fut);

        let client_res = client???;
        let gateway_res = gateway???;

        // ensure the created keys are the same
        assert_eq!(client_res, gateway_res);
        assert_eq!(client_res.negotiated_protocol, CURRENT_PROTOCOL_VERSION);

        Ok(())
    }

    #[tokio::test]
    async fn protocol_downgrade() -> anyhow::Result<()> {
        let (client, gateway) = setup();

        let handshake_client = client_handshake(
            client.rng,
            client.socket,
            client.keys,
            *gateway.keys.public_key(),
            Some(CURRENT_PROTOCOL_VERSION + 42),
            ShutdownToken::default(),
        );

        let client_fut = handshake_client.spawn_timeboxed();
        // we need to receive the first message so that it could be propagated to the gateway side of the handshake
        let init_msg = gateway.socket.get_handshake_init_data().await?;

        let handshake_gateway = gateway_handshake(
            gateway.rng,
            gateway.socket,
            gateway.keys,
            init_msg,
            Some(CURRENT_PROTOCOL_VERSION + 42),
            ShutdownToken::default(),
        );

        let gateway_fut = handshake_gateway.spawn_timeboxed();
        let (client, gateway) = join!(client_fut, gateway_fut);

        let client_res = client???;
        let gateway_res = gateway???;

        // ensure the created keys are the same
        assert_eq!(client_res, gateway_res);

        // and the protocol got downgraded for both parties
        assert_eq!(client_res.negotiated_protocol, CURRENT_PROTOCOL_VERSION);

        Ok(())
    }

    #[tokio::test]
    async fn protocol_upgrade() -> anyhow::Result<()> {
        let (client, gateway) = setup();

        let handshake_client = client_handshake(
            client.rng,
            client.socket,
            client.keys,
            *gateway.keys.public_key(),
            None,
            ShutdownToken::default(),
        );

        let client_fut = handshake_client.spawn_timeboxed();

        // we need to receive the first message so that it could be propagated to the gateway side of the handshake
        let init_msg = gateway.socket.get_handshake_init_data().await?;

        let handshake_gateway = gateway_handshake(
            gateway.rng,
            gateway.socket,
            gateway.keys,
            init_msg,
            None,
            ShutdownToken::default(),
        );

        let gateway_fut = handshake_gateway.spawn_timeboxed();
        let (client, gateway) = join!(client_fut, gateway_fut);

        let client_res = client???;
        let gateway_res = gateway???;

        // ensure the created keys are the same
        assert_eq!(client_res, gateway_res);

        // and the protocol got upgraded to the first known version
        assert_eq!(client_res.negotiated_protocol, INITIAL_PROTOCOL_VERSION);

        Ok(())
    }
}
