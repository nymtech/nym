use std::time::Duration;

use auth::{AuthLayer, TokenProvider};
use tonic::transport::{Channel, ClientTlsConfig, Uri};
use tower::ServiceBuilder;
use nymvpn_types::{
    location::Location,
    nymvpn_server::{
        Accepted, AddDeviceRequest, AddDeviceResponse, ClientConnected, EndSession, Ended,
        NewSession, VpnSessionStatus, VpnSessionStatusRequest,
    },
};

pub mod proto {
    tonic::include_proto!("nymvpn");
}

pub mod auth;
pub mod conversions;

pub const REQUEST_TIMEOUT_SECS: Duration = Duration::from_secs(60);

pub type NymvpnServiceClient<P> = proto::nymvpn_service_client::NymvpnServiceClient<AuthLayer<P>>;
pub type NymvpnServiceNoAuthClient =
    proto::nymvpn_service_client::NymvpnServiceClient<tonic::transport::Channel>;

async fn create_channel() -> Result<tonic::transport::Channel, tonic::transport::Error> {
    let tls = ClientTlsConfig::new();

    let api_host_port = nymvpn_config::config().grpc_api_host_port();
    //todo: use Uri type in config to avoid parsing panic?
    let uri: Uri = api_host_port.parse().expect("Failed to parse server uri");

    let channel = Channel::builder(uri)
        .tls_config(tls)?
        .keep_alive_while_idle(true)
        .http2_keep_alive_interval(Duration::from_secs(15))
        .tcp_keepalive(Some(Duration::from_secs(15)))
        .timeout(REQUEST_TIMEOUT_SECS)
        .connect()
        .await?;

    Ok(channel)
}

async fn channel_with_auth<P: TokenProvider>(
    token_provider: P,
) -> Result<AuthLayer<P>, tonic::transport::Error> {
    let channel = create_channel().await?;

    let channel = ServiceBuilder::new()
        .layer_fn(|inner| AuthLayer::new(token_provider.clone(), inner))
        .service(channel);

    Ok(channel)
}

fn backoff() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoffBuilder::new()
        .with_max_elapsed_time(Some(Duration::from_secs(5)))
        .build()
}

pub async fn new_nymvpn_service_client<P: TokenProvider + 'static>(
    token_provider: P,
) -> Result<NymvpnServiceClient<P>, tonic::transport::Error> {
    let channel = backoff::future::retry(backoff(), || async {
        let channel = channel_with_auth(token_provider.clone()).await?;
        Ok(channel)
    })
    .await
    .map_err(|e| e.into())?;

    Ok(proto::nymvpn_service_client::NymvpnServiceClient::new(
        channel,
    ))
}

pub async fn new_nymvpn_service_no_auth_client(
) -> Result<NymvpnServiceNoAuthClient, tonic::transport::Error> {
    let channel = backoff::future::retry(backoff(), || async {
        let channel = create_channel().await?;
        Ok(channel)
    })
    .await
    .map_err(|e| e.into())?;

    Ok(proto::nymvpn_service_client::NymvpnServiceClient::new(
        channel,
    ))
}

pub struct ServerApi<P: TokenProvider + 'static> {
    client: NymvpnServiceClient<P>,
}

impl<P: TokenProvider + 'static> ServerApi<P> {
    pub async fn new(token_provider: P) -> Result<Self, tonic::transport::Error> {
        Ok(Self {
            client: new_nymvpn_service_client(token_provider).await?,
        })
    }

    pub async fn list_locations(&mut self) -> Result<Vec<Location>, tonic::Status> {
        self.client
            .list_locations(())
            .await
            .map(|response| response.into_inner().into())
    }

    pub async fn new_session(
        &mut self,
        new_session: NewSession,
    ) -> Result<Accepted, tonic::Status> {
        let mut request = tonic::Request::new(new_session.into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client
            .new_vpn_session(request)
            .await
            .map(|response| response.into_inner().into())
    }

    pub async fn end_session(&mut self, end_session: EndSession) -> Result<Ended, tonic::Status> {
        let mut request = tonic::Request::new(end_session.into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client
            .end_vpn_session(request)
            .await
            .map(|response| response.into_inner().into())
    }

    pub async fn client_connected(
        &mut self,
        client_connected: ClientConnected,
    ) -> Result<(), tonic::Status> {
        let mut request = tonic::Request::new(client_connected.into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client
            .connected(request)
            .await
            .map(|response| response.into_inner())
    }

    pub async fn get_status(
        &mut self,
        vpn_session_status_request: VpnSessionStatusRequest,
    ) -> Result<VpnSessionStatus, tonic::Status> {
        let mut request = tonic::Request::new(vpn_session_status_request.into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client
            .get_status(request)
            .await
            .map(|response| response.into_inner().into())
    }

    pub async fn sign_out(&mut self) -> Result<(), tonic::Status> {
        let mut request = tonic::Request::new(().into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client.sign_out(request).await.map(|_| ())
    }

    pub async fn latest_app_version(&mut self) -> Result<String, tonic::Status> {
        let mut request = tonic::Request::new(().into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client
            .latest_app_version(request)
            .await
            .map(|response| response.into_inner())
    }
}

pub struct ServerApiNoAuth {
    client: NymvpnServiceNoAuthClient,
}

impl ServerApiNoAuth {
    pub async fn new() -> Result<Self, tonic::transport::Error> {
        Ok(Self {
            client: new_nymvpn_service_no_auth_client().await?,
        })
    }

    pub async fn add_device(
        &mut self,
        add_device_request: AddDeviceRequest,
    ) -> Result<AddDeviceResponse, tonic::Status> {
        let mut request = tonic::Request::new(add_device_request.into());
        request.set_timeout(REQUEST_TIMEOUT_SECS);
        self.client
            .add_device(request)
            .await
            .map(|response| response.into_inner().into())
    }
}
