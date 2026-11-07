// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use nym_crypto::asymmetric::ed25519;
use nym_node_requests::api::v1::gateway::models::{self, SignedBridgeClientParams};
use std::path::PathBuf;
use tower_http::services::ServeFile;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::Bridges>,
}

pub(crate) fn routes(config: Config) -> Router<AppState> {
    if let Some(cfg) = config.details {
        let path = PathBuf::from(&cfg.client_params_path);
        Router::new()
            .route_service("/client-params", ServeFile::new(&path))
            .route(
                "/signed-client-params",
                get(move |State(state): State<AppState>| {
                    signed_client_params(
                        path.clone(),
                        state.static_information.ed25519_identity_keys.clone(),
                    )
                }),
            )
    } else {
        Router::new()
    }
}

/// Returns the bridge client parameters file as one string, signed with the node identity key.
#[utoipa::path(
    get,
    path = "/signed-client-params",
    context_path = "/api/v1/bridges",
    tag = "v1 / Bridges",
    responses(
        (status = 200, content((SignedBridgeClientParams = "application/json"))),
        (status = 404, description = "no bridge client parameters on this node")
    )
)]
pub(crate) async fn signed_client_params(
    path: PathBuf,
    keys: std::sync::Arc<ed25519::KeyPair>,
) -> Result<Json<SignedBridgeClientParams>, StatusCode> {
    let payload = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(SignedBridgeClientParams::new(
        payload,
        keys.private_key(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;
    use std::sync::Arc;

    #[tokio::test]
    async fn signed_client_params_is_404_without_file_and_verifiable_with_it() {
        let keys = Arc::new(ed25519::KeyPair::new(&mut deterministic_rng()));
        let path = std::env::temp_dir().join(format!("signed-params-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let missing = signed_client_params(path.clone(), keys.clone()).await;
        assert_eq!(missing.unwrap_err(), StatusCode::NOT_FOUND);

        let file = "{\"transports_v2\": [ ]}\n";
        std::fs::write(&path, file).unwrap();
        let Json(signed) = signed_client_params(path.clone(), keys.clone())
            .await
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(signed.payload, file);
        assert!(signed.verify(keys.public_key()));
    }
}
