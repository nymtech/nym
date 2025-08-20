// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::error::HandshakeError;
use crate::registration::handshake::state::State;
use crate::SharedGatewayKey;
use futures::future::BoxFuture;
use futures::{Sink, Stream};
use nym_crypto::asymmetric::ed25519;
use rand::{CryptoRng, RngCore};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tungstenite::{Error as WsError, Message as WsMessage};

#[cfg(not(target_arch = "wasm32"))]
use nym_task::TaskClient;

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
    handshake_future: BoxFuture<'a, Result<SharedGatewayKey, HandshakeError>>,
}

impl Future for GatewayHandshake<'_> {
    type Output = Result<SharedGatewayKey, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}

pub fn client_handshake<'a, S, R>(
    rng: &'a mut R,
    ws_stream: &'a mut S,
    identity: &'a ed25519::KeyPair,
    gateway_pubkey: ed25519::PublicKey,
    expects_credential_usage: bool,
    derive_aes256_gcm_siv_key: bool,
    #[cfg(not(target_arch = "wasm32"))] shutdown: TaskClient,
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
        #[cfg(not(target_arch = "wasm32"))]
        shutdown,
    )
    .with_credential_usage(expects_credential_usage)
    .with_aes256_gcm_siv_key(derive_aes256_gcm_siv_key);

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
    shutdown: TaskClient,
) -> GatewayHandshake<'a>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
    R: CryptoRng + RngCore + Send,
{
    let state = State::new(rng, ws_stream, identity, None, shutdown);
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
    use crate::ClientControlRequest;
    use anyhow::Context;
    use futures::StreamExt;
    use nym_test_utils::helpers::u64_seeded_rng;
    use nym_test_utils::mocks::stream_sink::mock_streams;
    use nym_test_utils::traits::{Leak, Timeboxed, TimeboxedSpawnable};
    use std::time::Duration;
    use tokio::join;
    use tokio::time::timeout;
    use tungstenite::Message;

    #[tokio::test]
    async fn basic_handshake() -> anyhow::Result<()> {
        use anyhow::Context as _;

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

        let handshake_client = client_handshake(
            client_rng,
            client_ws,
            client_keys,
            *gateway_keys.public_key(),
            false,
            true,
            TaskClient::dummy(),
        );

        let client_fut = handshake_client.spawn_timeboxed();

        // we need to receive the first message so that it could be propagated to the gateway side of the handshake
        let ClientControlRequest::RegisterHandshakeInitRequest {
            protocol_version: _,
            data,
        } = (gateway_ws.next())
            .timeboxed()
            .await
            .context("timeout")?
            .context("no message!")??
            .into_text()?
            .parse::<ClientControlRequest>()?
        else {
            panic!("bad message")
        };

        let init_msg = data;

        let handshake_gateway = gateway_handshake(
            gateway_rng,
            gateway_ws,
            gateway_keys,
            init_msg,
            TaskClient::dummy(),
        );

        let gateway_fut = handshake_gateway.spawn_timeboxed();
        let (client, gateway) = join!(client_fut, gateway_fut);

        let client_key = client???;
        let gateway_key = gateway???;

        // ensure the created keys are the same
        assert_eq!(client_key, gateway_key);

        Ok(())
    }
}
