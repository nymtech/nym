use crate::accounting::{NetworkAccount, NetworkAccountStats, NodeStats};
use crate::handlers::{
    accounting_handler, all_nodes_stats_handler, graph_handler, mermaid_handler, mix_dot_handler,
    node_stats_handler, recv_handler, send_handler, sent_handler, stats_handler, FragmentsReceived,
    FragmentsSent,
};
use axum::routing::{get, post};
use axum::Router;
use log::{debug, info, warn};
use nym_sphinx::chunking::fragment::FragmentHeader;
use nym_sphinx::chunking::{ReceivedFragment, SentFragment};
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::ClientsWrapper;

pub struct HttpServer {
    listener: SocketAddr,
    cancel: CancellationToken,
    tcp_backlog: i32,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::accounting_handler,
        crate::handlers::graph_handler,
        crate::handlers::mermaid_handler,
        crate::handlers::mix_dot_handler,
        crate::handlers::node_stats_handler,
        crate::handlers::recv_handler,
        crate::handlers::send_handler,
        crate::handlers::sent_handler,
        crate::handlers::all_nodes_stats_handler,
    ),
    components(schemas(
        FragmentHeader,
        FragmentsReceived,
        FragmentsSent,
        NetworkAccount,
        NetworkAccountStats,
        NodeStats,
        ReceivedFragment,
        SentFragment,
    ))
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    clients: ClientsWrapper,
}

impl AppState {
    pub fn clients(&self) -> &ClientsWrapper {
        &self.clients
    }
}

impl HttpServer {
    pub fn new(listener: SocketAddr, cancel: CancellationToken, tcp_backlog: i32) -> Self {
        HttpServer {
            listener,
            cancel,
            tcp_backlog,
        }
    }

    pub async fn run(self, clients: ClientsWrapper) -> anyhow::Result<()> {
        let n_clients = clients.read().await.len();
        let state = AppState { clients };
        let app = Router::new()
            .route("/v1/send", post(send_handler))
            .merge(SwaggerUi::new("/v1/ui").url("/v1/docs/openapi.json", ApiDoc::openapi()))
            .route("/v1/accounting", get(accounting_handler))
            .route("/v1/sent", get(sent_handler))
            .route("/v1/dot/:mix_id", get(mix_dot_handler))
            .route("/v1/dot", get(graph_handler))
            .route("/v1/mermaid", get(mermaid_handler))
            .route("/v1/stats", get(stats_handler))
            .route("/v1/node_stats/:mix_id", get(node_stats_handler))
            .route("/v1/node_stats", get(all_nodes_stats_handler))
            .route("/v1/received", get(recv_handler))
            .layer(
                ServiceBuilder::new()
                    // Add request tracing
                    .layer(
                        TraceLayer::new_for_http()
                            .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                                debug!("[HTTP_REQUEST] New connection accepted");
                            })
                            .on_failure(|error: tower_http::classify::ServerErrorsFailureClass, latency: std::time::Duration, _span: &tracing::Span| {
                                warn!("[HTTP_ERROR] Request failed with error: {:?}, latency: {:?}", error, latency);
                            })
                    )
                    // Add a timeout layer to prevent hanging connections
                    .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
            )
            .with_state(state);
        // Configure socket with higher backlog to handle more concurrent connections
        let socket = socket2::Socket::new(
            socket2::Domain::for_address(self.listener),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;

        // Enable SO_REUSEADDR to avoid "Address already in use" errors
        socket.set_reuse_address(true)?;

        // Set a higher backlog (default is often 128)
        socket.bind(&self.listener.into())?;
        socket.listen(self.tcp_backlog)?; // Use configurable backlog

        let listener = tokio::net::TcpListener::from_std(socket.into())?;

        let server_future =
            axum::serve(listener, app).with_graceful_shutdown(self.cancel.cancelled_owned());

        info!("##########################################################################################");
        info!("######################### HTTP server running on {} with {} clients ############################################", self.listener, n_clients);
        info!("######################### TCP backlog set to {} connections ############################################", self.tcp_backlog);
        info!("##########################################################################################");
        info!("[HTTP_SERVER] Server started and ready to accept connections");

        server_future.await?;

        Ok(())
    }
}
