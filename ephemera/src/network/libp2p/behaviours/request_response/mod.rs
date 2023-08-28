use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite};
use libp2p::request_response;
use log::trace;
use serde::{Deserialize, Serialize};

use crate::broadcast::RbMsg;
use crate::utilities::codec::varint_async::{read_length_prefixed, write_length_prefixed};
use crate::utilities::id::EphemeraId;

#[derive(Clone)]
pub(crate) struct RbMsgMessagesCodec;

impl RbMsgMessagesCodec {}

#[derive(Clone)]
pub(crate) struct RbMsgProtocol;

impl request_response::ProtocolName for RbMsgProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/ephemera/reliable_broadcast/1.0.0".as_bytes()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RbMsgResponse {
    pub(crate) id: EphemeraId,
}

impl RbMsgResponse {
    pub(crate) fn new(id: EphemeraId) -> Self {
        Self { id }
    }
}

#[async_trait]
impl request_response::Codec for RbMsgMessagesCodec {
    type Protocol = RbMsgProtocol;
    type Request = RbMsg;
    type Response = RbMsgResponse;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> Result<Self::Request, std::io::Error>
    where
        T: AsyncRead + Unpin + Send,
    {
        //FIXME: max size
        let data = read_length_prefixed(io, 1024 * 1024).await?;
        //FIXME: switch to binary
        let msg = serde_json::from_slice(&data)?;
        trace!("Received request {:?}", msg);
        Ok(msg)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        //FIXME: max size
        let response = read_length_prefixed(io, 1024 * 1024).await?;
        //FIXME: switch to binary
        let response = serde_json::from_slice(&response)?;
        trace!("Received response {:?}", response);
        Ok(response)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> Result<(), std::io::Error>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req).unwrap();
        write_length_prefixed(io, data).await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let response = serde_json::to_vec(&response).unwrap();
        write_length_prefixed(io, response).await?;
        Ok(())
    }
}
