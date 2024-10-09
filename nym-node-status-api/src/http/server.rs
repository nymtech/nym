use axum::Router;
use core::net::SocketAddr;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::{
    db::DbPool,
    http::{api::RouterBuilder, state::AppState},
};

/// Return handles that allow for graceful shutdown of server + awaiting its
/// background tokio task
pub(crate) async fn start_http_api(
    db_pool: DbPool,
    http_port: u16,
    nym_http_cache_ttl: u64,
) -> anyhow::Result<ShutdownHandles> {
    let router_builder = RouterBuilder::with_default_routes();

    let state = AppState::new(db_pool, nym_http_cache_ttl);
    let router = router_builder.with_state(state);

    // TODO dz do we need this to be configurable?
    let bind_addr = format!("0.0.0.0:{}", http_port);
    let server = router.build_server(bind_addr).await?;

    Ok(start_server(server))
}

fn start_server(server: HttpServer) -> ShutdownHandles {
    // one copy is stored to trigger a graceful shutdown later
    let shutdown_button = CancellationToken::new();
    // other copy is given to server to listen for a shutdown
    let shutdown_receiver = shutdown_button.clone();
    let shutdown_receiver = shutdown_receiver.cancelled_owned();

    let server_handle = tokio::spawn(async move { server.run(shutdown_receiver).await });

    ShutdownHandles {
        server_handle,
        shutdown_button,
    }
}

pub(crate) struct ShutdownHandles {
    server_handle: JoinHandle<std::io::Result<()>>,
    shutdown_button: CancellationToken,
}

impl ShutdownHandles {
    /// Send graceful shutdown signal to server and wait for server task to complete
    pub(crate) async fn shutdown(self) -> anyhow::Result<()> {
        self.shutdown_button.cancel();

        match self.server_handle.await {
            Ok(Ok(_)) => {
                tracing::info!("HTTP server shut down without errors");
            }
            Ok(Err(err)) => {
                tracing::error!("HTTP server terminated with: {err}");
                anyhow::bail!(err)
            }
            Err(err) => {
                tracing::error!("Server task panicked: {err}");
            }
        };

        Ok(())
    }
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
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(receiver)
        .await
    }
}
