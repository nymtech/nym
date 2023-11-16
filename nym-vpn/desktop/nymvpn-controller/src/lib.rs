use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use auth::Auth;
use parity_tokio_ipc::Endpoint as IpcEndpoint;
use prost_types::Timestamp;
use thiserror::Error;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::ReadBuf;
use tonic::transport::server::Connected;
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;
use tonic::transport::Server;
use tonic::transport::Uri;
use tower::service_fn;

pub mod proto {
    tonic::include_proto!("nymvpn.controller");
}

pub mod auth;
pub mod conversions;

use chrono::{TimeZone, Utc};
pub use proto::controller_service_server::{ControllerService, ControllerServiceServer};
use nymvpn_types::DateTimeUtc;

use crate::auth::ControllerAuthLayer;

pub type ControllerServiceClient =
    proto::controller_service_client::ControllerServiceClient<Channel>;

pub type GrpcServerJoinHandle = tokio::task::JoinHandle<Result<(), ControllerError>>;

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("{0}")]
    TonicTransportError(tonic::transport::Error),
    #[error("security attributes error {0:#?}")]
    SecurityAttributesError(std::io::Error),
    #[error("incoming connection error {0:#?}")]
    IncomingConnectionError(std::io::Error),
}

pub async fn new_grpc_client() -> Result<ControllerServiceClient, ControllerError> {
    let ipc_path = nymvpn_config::config().socket_path();

    // URI is unused
    let channel = TonicEndpoint::from_static("http://[::]:50051")
        .connect_with_connector(service_fn(move |_: Uri| {
            IpcEndpoint::connect(ipc_path.clone())
        }))
        .await
        .map_err(ControllerError::TonicTransportError)?;

    Ok(ControllerServiceClient::new(channel))
}

pub async fn spawn_grpc_server<S, P, F>(
    service: S,
    auth: P,
    shutdown: F,
) -> std::result::Result<GrpcServerJoinHandle, ControllerError>
where
    S: proto::controller_service_server::ControllerService,
    F: Future<Output = ()> + Send + 'static,
    P: Auth + 'static,
{
    use futures::stream::TryStreamExt;
    use parity_tokio_ipc::SecurityAttributes;

    let socket_path = nymvpn_config::config().socket_path();

    let mut endpoint = IpcEndpoint::new(socket_path.to_string_lossy().to_string());
    endpoint.set_security_attributes(
        SecurityAttributes::allow_everyone_create()
            .map_err(ControllerError::SecurityAttributesError)?
            .set_mode(0o766)
            .map_err(ControllerError::SecurityAttributesError)?,
    );

    let incoming = endpoint
        .incoming()
        .map_err(ControllerError::IncomingConnectionError)?;

    Ok(tokio::spawn(async move {
        Server::builder()
            .layer(ControllerAuthLayer::new(auth))
            .add_service(ControllerServiceServer::new(service))
            .serve_with_incoming_shutdown(incoming.map_ok(StreamBox), shutdown)
            .await
            .map_err(ControllerError::TonicTransportError)
    }))
}

#[derive(Debug)]
struct StreamBox<T: AsyncRead + AsyncWrite>(pub T);

impl<T: AsyncRead + AsyncWrite> Connected for StreamBox<T> {
    type ConnectInfo = Option<()>;

    fn connect_info(&self) -> Self::ConnectInfo {
        None
    }
}
impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for StreamBox<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for StreamBox<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

pub fn timestamp_to_datetime_utc(timestamp: Option<Timestamp>) -> Result<DateTimeUtc, String> {
    let timestamp = timestamp.ok_or(format!("no timestamp"))?;
    let date_time_utc = match Utc.timestamp_opt(timestamp.seconds, timestamp.nanos as u32) {
        chrono::LocalResult::Single(dtu) => dtu,
        chrono::LocalResult::None => Err("invalid utc time none")?,
        chrono::LocalResult::Ambiguous(a, _b) => {
            //Err(format!("ambiguous utc time {a} {b}"))?
            a
        }
    };
    Ok(date_time_utc)
}

pub fn datetime_utc_to_timestamp(datetime_utc: DateTimeUtc) -> Timestamp {
    let seconds = datetime_utc.timestamp();
    let nanos = std::cmp::max(datetime_utc.timestamp_subsec_nanos() as i32, 0);
    prost_types::Timestamp { seconds, nanos }
}
