// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use self::client::ClientHandshake;
use self::error::HandshakeError;
use self::gateway::GatewayHandshake;
pub use self::shared_key::{SharedKeySize, SharedKeys};
use crypto::asymmetric::identity;
use futures::{Sink, Stream};
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use tungstenite::{Error as WsError, Message as WsMessage};

// for ease of use
pub const DEFAULT_RNG: OsRng = OsRng;

pub(crate) type WsItem = Result<WsMessage, WsError>;

mod client;
pub mod error;
mod gateway;
pub mod shared_key;
mod state;

// Note: the handshake is built on top of WebSocket, but in principle it shouldn't be too difficult
// to remove that restriction, by just changing Sink<WsMessage> and Stream<Item = WsMessage> into
// AsyncWrite and AsyncRead and slightly adjusting the implementation. But right now
// we do not need to worry about that.

pub async fn client_handshake<'a, S>(
    rng: &mut (impl RngCore + CryptoRng),
    ws_stream: &'a mut S,
    identity: &'a identity::KeyPair,
    gateway_pubkey: identity::PublicKey,
) -> Result<SharedKeys, HandshakeError>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
{
    ClientHandshake::new(rng, ws_stream, identity, gateway_pubkey).await
}

pub async fn gateway_handshake<'a, S>(
    rng: &mut (impl RngCore + CryptoRng),
    ws_stream: &'a mut S,
    identity: &'a identity::KeyPair,
    received_init_payload: Vec<u8>,
) -> Result<SharedKeys, HandshakeError>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
{
    GatewayHandshake::new(rng, ws_stream, identity, received_init_payload).await
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
