use crate::ticketbook_manager::state::TicketbookManagerState;
use crate::{
    db,
    http::{api::RouterBuilder, state::AppState},
    monitor::{DelegationsCache, NodeGeoCache},
};
use axum::Router;
use core::net::SocketAddr;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_task::ShutdownTracker;
use std::sync::Arc;
use tokio::{net::TcpListener, sync::RwLock};

/// Return handles that allow for graceful shutdown of server + awaiting its
/// background tokio task
#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_http_api(
    storage: db::Storage,
    http_port: u16,
    nym_http_cache_ttl: u64,
    agent_key_list: Vec<PublicKey>,
    agent_max_count: i64,
    agent_request_freshness_requirement: time::Duration,
    node_geocache: NodeGeoCache,
    node_delegations: Arc<RwLock<DelegationsCache>>,
    ticketbook_manager_state: TicketbookManagerState,
    shutdown_tracker: &ShutdownTracker,
) -> anyhow::Result<()> {
    let router_builder = RouterBuilder::with_default_routes();

    let state = AppState::new(
        storage,
        nym_http_cache_ttl,
        agent_key_list,
        agent_max_count,
        agent_request_freshness_requirement,
        node_geocache,
        node_delegations,
        ticketbook_manager_state,
    )
    .await;
    let router = router_builder.with_state(state);

    let bind_addr = format!("0.0.0.0:{http_port}");
    tracing::info!("Binding server to {bind_addr}");
    let server = router.build_server(bind_addr).await?;
    let shutdown = shutdown_tracker.clone_shutdown_token().cancelled_owned();

    shutdown_tracker.spawn(async move {
        axum::serve(
            server.listener,
            server
                .router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown)
        .await
    });

    Ok(())
}

pub(crate) struct HttpServer {
    router: Router,
    listener: TcpListener,
}

impl HttpServer {
    pub(crate) fn new(router: Router, listener: TcpListener) -> Self {
        Self { router, listener }
    }
}
