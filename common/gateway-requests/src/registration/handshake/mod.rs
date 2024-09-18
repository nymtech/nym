// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::error::HandshakeError;
use crate::registration::handshake::state::State;
use futures::future::BoxFuture;
use futures::{Sink, Stream};
use nym_crypto::asymmetric::identity;
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
mod shared_key;
mod state;

pub use self::shared_key::legacy::{LegacySharedKeySize, LegacySharedKeys};
pub use self::shared_key::{
    SharedGatewayKey, SharedKeyConversionError, SharedKeyUsageError, SharedSymmetricKey,
};

// realistically even 32bit would have sufficed, so 128 is definitely enough
pub const KDF_SALT_LENGTH: usize = 16;

// Note: the handshake is built on top of WebSocket, but in principle it shouldn't be too difficult
// to remove that restriction, by just changing Sink<WsMessage> and Stream<Item = WsMessage> into
// AsyncWrite and AsyncRead and slightly adjusting the implementation. But right now
// we do not need to worry about that.

pub struct GatewayHandshake<'a> {
    handshake_future: BoxFuture<'a, Result<SharedGatewayKey, HandshakeError>>,
}

impl<'a> Future for GatewayHandshake<'a> {
    type Output = Result<SharedGatewayKey, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}

pub fn client_handshake<'a, S, R>(
    rng: &'a mut R,
    ws_stream: &'a mut S,
    identity: &'a identity::KeyPair,
    gateway_pubkey: identity::PublicKey,
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
    identity: &'a identity::KeyPair,
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
