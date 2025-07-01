use axum::Router;
use core::net::SocketAddr;
use nym_task::ShutdownToken;
use tokio::net::TcpListener;

use crate::{
    http::{api::RouterBuilder, state::AppState},
    network_view::NetworkView,
    storage::StatisticsStorage,
};

pub(crate) async fn build_http_api(
    storage: StatisticsStorage,
    cached_network: NetworkView,
    http_port: u16,
) -> anyhow::Result<HttpServer> {
    let router_builder = RouterBuilder::with_default_routes();

    let state = AppState::new(storage, cached_network).await;
    let router = router_builder.with_state(state);

    let bind_addr = format!("0.0.0.0:{http_port}");
    tracing::info!("Binding server to {bind_addr}");

    router.build_server(bind_addr).await
}

pub(crate) struct HttpServer {
    router: Router,
    listener: TcpListener,
}

impl HttpServer {
    pub(crate) fn new(router: Router, listener: TcpListener) -> Self {
        Self { router, listener }
    }

    pub(crate) async fn run(self, shutdown_token: ShutdownToken) -> std::io::Result<()> {
        // into_make_service_with_connect_info allows us to see client ip address
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { shutdown_token.cancelled().await })
        .await
    }
}
