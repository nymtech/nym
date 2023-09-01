use std::future::Future;
use std::iter;
use std::pin::Pin;

use asynchronous_codec::{Decoder, Encoder, Framed};
use bytes::BytesMut;
use futures::{AsyncRead, AsyncWrite};
use futures_util::future;
use libp2p::core::UpgradeInfo;
use libp2p::{InboundUpgrade, OutboundUpgrade};
use log::trace;
use serde::{Deserialize, Serialize};

use crate::utilities::codec::varint_bytes::{read_length_prefixed, write_length_prefixed};

pub const PROTOCOL_NAME: &[u8] = b"/ephemera/membership/1.0.0";

pub(crate) struct Protocol;

impl UpgradeInfo for Protocol {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(PROTOCOL_NAME)
    }
}

impl<C> InboundUpgrade<C> for Protocol
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = Framed<C, MembershipCodec>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: C, _: Self::Info) -> Self::Future {
        trace!(
            "Inbound upgrade for protocol: {}",
            String::from_utf8_lossy(PROTOCOL_NAME)
        );
        Box::pin(future::ok(Framed::new(socket, MembershipCodec {})))
    }
}

impl<C> OutboundUpgrade<C> for Protocol
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = Framed<C, MembershipCodec>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, socket: C, _: Self::Info) -> Self::Future {
        trace!(
            "Outbound upgrade for protocol: {}",
            String::from_utf8_lossy(PROTOCOL_NAME)
        );
        Box::pin(future::ok(Framed::new(socket, MembershipCodec {})))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ProtocolMessage {
    Sync,
}

pub(crate) struct MembershipCodec {}

impl Encoder for MembershipCodec {
    type Item = ProtocolMessage;
    type Error = anyhow::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        //FIXME: switch to binary
        let data = serde_json::to_vec(&item).unwrap();
        write_length_prefixed(dst, data);
        Ok(())
    }
}

impl Decoder for MembershipCodec {
    type Item = ProtocolMessage;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }
        let data = read_length_prefixed(src, 1024 * 1024)?;
        match data {
            None => Ok(None),
            Some(data) => {
                //FIXME: switch to binary
                let msg = serde_json::from_slice(&data)?;
                Ok(msg)
            }
        }
    }
}
