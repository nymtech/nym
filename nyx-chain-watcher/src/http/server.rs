use axum::Router;
use core::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_util::sync::WaitForCancellationFutureOwned;

use crate::config::Config;
use crate::{
    db::DbPool,
    http::{api::RouterBuilder, state::AppState},
};

pub(crate) async fn build_http_api(
    db_pool: DbPool,
    config: &Config,
    http_port: u16,
) -> anyhow::Result<HttpServer> {
    let router_builder = RouterBuilder::with_default_routes();

    let watched_accounts = config
        .payment_watcher_config
        .watched_transfer_accounts()
        .iter()
        .map(|a| a.to_string())
        .collect();

    let state = AppState::new(db_pool, watched_accounts);
    let router = router_builder.with_state(state);

    let bind_addr = format!("0.0.0.0:{}", http_port);
    let server = router.build_server(bind_addr).await?;
    Ok(server)
}

pub(crate) struct HttpServer {
    router: Router,
    listener: TcpListener,
}

impl HttpServer {
    pub(crate) fn new(router: Router, listener: TcpListener) -> Self {
        Self { router, listener }
    }

    pub(crate) async fn run(self, receiver: WaitForCancellationFutureOwned) -> std::io::Result<()> {
        // into_make_service_with_connect_info allows us to see client ip address
        // in middleware, for logging, TLS, routing etc.
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(receiver)
        .await
    }
}
