// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::tests::build_dummy_ecash_state;
use crate::node_families::cache::{CachedFamily, CachedFamilyMember, NodeFamiliesCacheData};
use crate::node_families::handlers;
use crate::support::caching::cache::SharedCache;
use crate::support::config;
use crate::support::http::state::test_helpers::build_app_state;
use crate::support::storage::NymApiStorage;
use axum::Router;
use axum_test::http::StatusCode;
use axum_test::TestServer;
use cosmwasm_std::Coin;
use nym_api_requests::models::node_families::{
    NodeFamily, NodeFamilyForNodeResponse, NodeFamilyResponse, NodeStakeInformation,
};
use nym_api_requests::pagination::PaginatedResponse;
use nym_mixnet_contract_common::NodeId;
use nym_node_families_contract_common::NodeFamilyId;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use time::OffsetDateTime;

struct NodeFamiliesTestFixture {
    axum: TestServer,
    cache: SharedCache<NodeFamiliesCacheData>,
}

impl NodeFamiliesTestFixture {
    /// Build a test server with the node-families router mounted and an empty
    /// (but initialised) shared cache. Use [`seed`] to populate it.
    async fn new() -> Self {
        let storage = NymApiStorage::init_in_memory().await.unwrap();

        let cache = SharedCache::<NodeFamiliesCacheData>::new_with_value(Default::default());

        // node-families handlers don't read `AppState.ecash_state`, but
        // `AppState` requires one; we just need a valid construction.
        let mut cfg = config::Config::new("test");
        cfg.ecash_signer.enabled = false;
        let bundle = build_dummy_ecash_state(&cfg, storage.clone(), [7u8; 32]).await;

        let app_state = build_app_state(
            storage,
            bundle.ecash_state,
            bundle.real_client,
            cache.clone(),
        );

        let server = TestServer::new(
            Router::new()
                .nest("/v1/node-families", handlers::routes())
                .with_state(app_state),
        )
        .unwrap();

        NodeFamiliesTestFixture {
            axum: server,
            cache,
        }
    }

    /// Replace the cached data with the provided snapshot.
    async fn seed(&self, data: NodeFamiliesCacheData) {
        // try_overwrite_old_value swaps the entire cached value
        self.cache
            .try_overwrite_old_value(data, "node-families-test")
            .await
            .ok();
    }
}

// ---------- fixtures ----------

fn stake_coin(amount: u128) -> Coin {
    Coin::new(amount, "unym")
}

fn member(node_id: NodeId, with_stake: Option<u128>) -> CachedFamilyMember {
    CachedFamilyMember {
        node_id,
        joined_at: OffsetDateTime::UNIX_EPOCH,
        bonding_height: Some(1),
        node_stake_information: with_stake.map(|amt| NodeStakeInformation {
            stake: stake_coin(amt),
            bond: stake_coin(amt),
            delegations: stake_coin(0),
            delegators: 0,
        }),
    }
}

fn family(id: NodeFamilyId, name: &str, members: Vec<CachedFamilyMember>) -> CachedFamily {
    CachedFamily {
        id,
        name: name.to_string(),
        description: format!("{name} description"),
        owner: format!("n1owner{id}"),
        average_node_age: Duration::ZERO,
        total_stake: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
        members,
        pending_invitations: Vec::new(),
    }
}

fn snapshot(families: Vec<CachedFamily>) -> NodeFamiliesCacheData {
    let mut family_by_member: HashMap<NodeId, NodeFamilyId> = HashMap::new();
    let mut families_map: BTreeMap<NodeFamilyId, CachedFamily> = BTreeMap::new();
    for family in families {
        for m in &family.members {
            family_by_member.insert(m.node_id, family.id);
        }
        families_map.insert(family.id, family);
    }
    NodeFamiliesCacheData {
        families: families_map,
        family_by_member,
        block_timestamps: HashMap::new(),
    }
}

// ---------- /v1/node-families (list) ----------

#[tokio::test]
async fn list_returns_empty_when_cache_has_no_families() {
    let fx = NodeFamiliesTestFixture::new().await;

    let res = fx.axum.get("/v1/node-families").await;
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: PaginatedResponse<NodeFamily> = res.json();
    assert_eq!(body.pagination.total, 0);
    assert_eq!(body.pagination.size, 0);
    assert_eq!(body.pagination.page, 0);
    assert!(body.data.is_empty());
}

#[tokio::test]
async fn list_returns_every_seeded_family() {
    let fx = NodeFamiliesTestFixture::new().await;
    fx.seed(snapshot(vec![
        family(1, "alpha", vec![member(10, Some(100))]),
        family(2, "beta", vec![member(20, None)]),
    ]))
    .await;

    let res = fx.axum.get("/v1/node-families").await;
    let body: PaginatedResponse<NodeFamily> = res.json();
    assert_eq!(body.pagination.total, 2);
    let ids: Vec<_> = body.data.iter().map(|f| f.id).collect();
    assert_eq!(ids, vec![1, 2]);
}

#[tokio::test]
async fn list_paginates_by_offset_in_ascending_id_order() {
    let fx = NodeFamiliesTestFixture::new().await;
    let families: Vec<_> = (1u32..=5)
        .map(|i| family(i, &format!("f{i}"), vec![]))
        .collect();
    fx.seed(snapshot(families)).await;

    // page 0 / per_page 2 → ids [1, 2]
    let res = fx.axum.get("/v1/node-families?page=0&per_page=2").await;
    let page0: PaginatedResponse<NodeFamily> = res.json();
    assert_eq!(page0.pagination.total, 5);
    assert_eq!(page0.pagination.page, 0);
    assert_eq!(page0.pagination.size, 2);
    assert_eq!(
        page0.data.iter().map(|f| f.id).collect::<Vec<_>>(),
        vec![1, 2]
    );

    // page 1 / per_page 2 → ids [3, 4]
    let res = fx.axum.get("/v1/node-families?page=1&per_page=2").await;
    let page1: PaginatedResponse<NodeFamily> = res.json();
    assert_eq!(
        page1.data.iter().map(|f| f.id).collect::<Vec<_>>(),
        vec![3, 4]
    );

    // page 2 / per_page 2 → just id 5 on the last page
    let res = fx.axum.get("/v1/node-families?page=2&per_page=2").await;
    let page2: PaginatedResponse<NodeFamily> = res.json();
    assert_eq!(page2.pagination.size, 1);
    assert_eq!(page2.data.iter().map(|f| f.id).collect::<Vec<_>>(), vec![5]);

    // page 3 / per_page 2 → past the end; data empty but total still reported
    let res = fx.axum.get("/v1/node-families?page=3&per_page=2").await;
    let page3: PaginatedResponse<NodeFamily> = res.json();
    assert_eq!(page3.pagination.total, 5);
    assert_eq!(page3.pagination.size, 0);
    assert!(page3.data.is_empty());
}

// ---------- /v1/node-families/{family_id} ----------

#[tokio::test]
async fn get_family_by_id_returns_some_on_hit() {
    let fx = NodeFamiliesTestFixture::new().await;
    fx.seed(snapshot(vec![family(7, "seven", vec![])])).await;

    let res = fx.axum.get("/v1/node-families/7").await;
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: NodeFamilyResponse = res.json();
    let family = body.family.expect("family should be present");
    assert_eq!(family.id, 7);
    assert_eq!(family.name, "seven");
}

#[tokio::test]
async fn get_family_by_id_returns_none_on_miss() {
    let fx = NodeFamiliesTestFixture::new().await;
    fx.seed(snapshot(vec![family(1, "alpha", vec![])])).await;

    let res = fx.axum.get("/v1/node-families/999").await;
    // still 200 — `family: None` signals absence
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: NodeFamilyResponse = res.json();
    assert!(body.family.is_none());
}

// ---------- /v1/node-families/by-node/{node_id} ----------

#[tokio::test]
async fn get_family_for_node_returns_some_on_hit() {
    let fx = NodeFamiliesTestFixture::new().await;
    fx.seed(snapshot(vec![family(
        3,
        "gamma",
        vec![member(42, None), member(43, None)],
    )]))
    .await;

    let res = fx.axum.get("/v1/node-families/by-node/42").await;
    let body: NodeFamilyForNodeResponse = res.json();
    assert_eq!(body.node_id, 42);
    let family = body.family.expect("expected to find the family");
    assert_eq!(family.id, 3);
}

#[tokio::test]
async fn get_family_for_node_returns_none_on_miss() {
    let fx = NodeFamiliesTestFixture::new().await;
    fx.seed(snapshot(vec![family(1, "alpha", vec![member(10, None)])]))
        .await;

    let res = fx.axum.get("/v1/node-families/by-node/9999").await;
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: NodeFamilyForNodeResponse = res.json();
    assert_eq!(body.node_id, 9999);
    assert!(body.family.is_none());
}
