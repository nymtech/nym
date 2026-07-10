// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! HTTP-level signer-fault harness (openspec change
//! `dvpn-signer-fault-http-harness`): drives the REAL
//! [`NyxdGlobalDataFetcher`] — real discovery, real HTTP client, real
//! try-each-API loop — against local mock nym-apis whose routes are
//! independently scriptable as healthy / erroring / **hanging** (the observed
//! mainnet outage: connection accepted, no response ever).
//!
//! Test naming convention: `characterize_*` tests assert CURRENT behavior of
//! the unfixed stack (however undesirable) — if a `common/`-level timeout ever
//! ships they will fail, which is the signal to flip them into guards.
//! Unprefixed tests are guards on desired behavior.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nym_api_requests::ecash::models::{
    AggregatedCoinIndicesSignatureResponse, AggregatedExpirationDateSignatureResponse,
    MasterVerificationKeyResponse, PartialExpirationDateSignatureResponse,
};
use nym_bandwidth_controller::CredentialPublicDataFetcher;
use nym_bandwidth_fetcher::NyxdGlobalDataFetcher;
use nym_sdk_session::TimeoutFetcher;
use nym_validator_client::nym_api::NymApiClientExt;

use support::fake_dkg::FakeDkg;
use support::http_harness::{FaultMode, MockNymApi};
use support::{TestEcash, EPOCH_ID};

const MVK_ROUTE: &str = "/v1/ecash/master-verification-key";
const COIN_ROUTE: &str = "/v1/ecash/aggregated-coin-indices-signatures";
const AGG_EXPIRATION_ROUTE: &str = "/v1/ecash/aggregated-expiration-date-signatures";
const PARTIAL_EXPIRATION_ROUTE: &str = "/v1/ecash/partial-expiration-date-signatures";

/// The full healthy route map for one mock signer, from the fixture set.
fn healthy_routes(ecash: &TestEcash) -> HashMap<String, FaultMode> {
    let (expiration_date, expiration_sigs) = ecash.expiration_date_signatures();
    let mut routes = HashMap::new();
    routes.insert(
        MVK_ROUTE.to_string(),
        FaultMode::Healthy(
            serde_json::to_string(&MasterVerificationKeyResponse {
                key: ecash.epoch_verification_key(EPOCH_ID).key,
            })
            .unwrap(),
        ),
    );
    routes.insert(
        COIN_ROUTE.to_string(),
        FaultMode::Healthy(
            serde_json::to_string(&AggregatedCoinIndicesSignatureResponse {
                epoch_id: EPOCH_ID,
                signatures: ecash.coin_index_signatures(EPOCH_ID).signatures,
            })
            .unwrap(),
        ),
    );
    routes.insert(
        AGG_EXPIRATION_ROUTE.to_string(),
        FaultMode::Healthy(
            serde_json::to_string(&AggregatedExpirationDateSignatureResponse {
                epoch_id: EPOCH_ID,
                expiration_date,
                signatures: expiration_sigs.clone(),
            })
            .unwrap(),
        ),
    );
    routes.insert(
        PARTIAL_EXPIRATION_ROUTE.to_string(),
        FaultMode::Healthy(
            serde_json::to_string(&PartialExpirationDateSignatureResponse {
                epoch_id: EPOCH_ID,
                expiration_date,
                signatures: expiration_sigs,
            })
            .unwrap(),
        ),
    );
    routes
}

/// A fetcher whose discovery resolves to the given mock signers.
fn fetcher_for(ecash: &TestEcash, apis: &[&MockNymApi]) -> NyxdGlobalDataFetcher<FakeDkg> {
    let signers = apis
        .iter()
        .enumerate()
        .map(|(i, api)| (api.url().to_string(), ecash.vk_share_bs58(i)));
    NyxdGlobalDataFetcher::new(Arc::new(FakeDkg::new(EPOCH_ID, ecash.threshold(), signers)))
}

/// Spec: "Real discovery resolves to mock servers" + fixture sanity — with an
/// all-healthy signer, the real fetcher discovers it via the DKG double and
/// fetches all three global-data kinds through the real HTTP stack.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn healthy_signer_serves_all_global_data_through_real_stack() {
    let ecash = TestEcash::new();
    let api = MockNymApi::spawn(healthy_routes(&ecash)).await;
    let fetcher = fetcher_for(&ecash, &[&api]);

    let vk = fetcher
        .fetch_master_verification_key(EPOCH_ID)
        .await
        .expect("master verification key");
    assert_eq!(vk.epoch_id, EPOCH_ID);

    let coins = fetcher
        .fetch_coin_index_signatures(EPOCH_ID)
        .await
        .expect("coin index signatures");
    assert!(!coins.signatures.is_empty());

    let (expiration_date, _) = ecash.expiration_date_signatures();
    let sigs = fetcher
        .fetch_expiration_date_signatures(expiration_date, EPOCH_ID)
        .await
        .expect("expiration date signatures");
    assert!(!sigs.signatures.is_empty());
}

/// Spec: "Reproduce the observed mainnet failure signature" — healthy siblings
/// but a hanging aggregated-expiration route: the UNMODIFIED fetch stack does
/// not complete within the characterization deadline (there is no timeout
/// anywhere below the trait seam).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn characterize_hanging_aggregated_expiration_blocks_raw_fetcher() {
    let ecash = TestEcash::new();
    let api = MockNymApi::spawn(healthy_routes(&ecash)).await;
    api.set(AGG_EXPIRATION_ROUTE, FaultMode::Hang);
    let fetcher = fetcher_for(&ecash, &[&api]);

    // Siblings still answer instantly…
    fetcher
        .fetch_master_verification_key(EPOCH_ID)
        .await
        .expect("siblings healthy");

    // …while the aggregated expiration fetch is still pending at the deadline.
    let (expiration_date, _) = ecash.expiration_date_signatures();
    let probe = tokio::time::timeout(
        Duration::from_secs(2),
        fetcher.fetch_expiration_date_signatures(expiration_date, EPOCH_ID),
    )
    .await;
    assert!(
        probe.is_err(),
        "characterization: the raw stack must still hang on this outage \
         (if this fails, a lower-level timeout shipped — flip this test into a guard)"
    );
}

/// Spec: "Bounded behavior with the timeout decorator layered on" — the SDK's
/// `TimeoutFetcher` converts the same hang into a bounded error, measured
/// end-to-end through the real HTTP stack.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn timeout_decorator_bounds_the_hang_end_to_end() {
    let ecash = TestEcash::new();
    let api = MockNymApi::spawn(healthy_routes(&ecash)).await;
    api.set(AGG_EXPIRATION_ROUTE, FaultMode::Hang);
    let bounded =
        TimeoutFetcher::with_timeout(fetcher_for(&ecash, &[&api]), Duration::from_millis(300));

    let (expiration_date, _) = ecash.expiration_date_signatures();
    let start = Instant::now();
    let err = bounded
        .fetch_expiration_date_signatures(expiration_date, EPOCH_ID)
        .await
        .expect_err("must be bounded");
    assert!(
        err.to_string().contains("ecash signers unresponsive"),
        "got: {err}"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "bounded well under the characterization deadline (took {:?})",
        start.elapsed()
    );
}

/// Spec: "Hanging first API with a healthy fallback" — multiple discovered
/// APIs, one hanging on the aggregated route. CHARACTERIZATION FINDING baked
/// into this guard: the SDK's per-fetch bound wraps the fetcher's WHOLE
/// try-each-API loop, so a single call that shuffles the hanging API first
/// times out WITHOUT reaching the healthy one — per-API fallback needs a
/// per-request bound inside `query_random_apis_until_success`
/// (`common/bandwidth-fetcher`), the improvement this harness exists to
/// motivate. What the SDK guarantees today, asserted here: every attempt is
/// bounded, and bounded RETRIES reach a healthy signer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_retries_reach_a_healthy_signer_despite_a_hanging_one() {
    let ecash = TestEcash::new();
    let hanging = MockNymApi::spawn(healthy_routes(&ecash)).await;
    hanging.set(AGG_EXPIRATION_ROUTE, FaultMode::Hang);
    let healthy_a = MockNymApi::spawn(healthy_routes(&ecash)).await;
    let healthy_b = MockNymApi::spawn(healthy_routes(&ecash)).await;
    let bounded = TimeoutFetcher::with_timeout(
        fetcher_for(&ecash, &[&hanging, &healthy_a, &healthy_b]),
        Duration::from_millis(300),
    );

    let (expiration_date, _) = ecash.expiration_date_signatures();
    let mut attempts = 0;
    let succeeded = loop {
        attempts += 1;
        let start = Instant::now();
        let result = bounded
            .fetch_expiration_date_signatures(expiration_date, EPOCH_ID)
            .await;
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "every attempt must be bounded (attempt {attempts} took {:?})",
            start.elapsed()
        );
        if result.is_ok() {
            break true;
        }
        // 2-in-3 chance per attempt of shuffling a healthy API first;
        // P(20 consecutive failures) ≈ 3e-10 — deterministic in practice.
        if attempts >= 20 {
            break false;
        }
    };
    assert!(
        succeeded,
        "bounded retries must eventually reach a healthy signer"
    );
    eprintln!("multi-api: succeeded after {attempts} bounded attempt(s)");
}

/// Spec: "K live signers at or above threshold" — the partial-availability
/// probe (assertion-light data collection for the client-side
/// partial-signature-aggregation decision): with the aggregated route dead on
/// every signer but K ≥ threshold serving partials, sufficient partials are
/// retrievable directly.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn threshold_probe_partials_survive_aggregated_outage() {
    let ecash = TestEcash::new();
    // All three signers: aggregated route dead everywhere; partials healthy on
    // two (== threshold), erroring on the third.
    let apis = [
        MockNymApi::spawn(healthy_routes(&ecash)).await,
        MockNymApi::spawn(healthy_routes(&ecash)).await,
        MockNymApi::spawn(healthy_routes(&ecash)).await,
    ];
    for api in &apis {
        api.set(AGG_EXPIRATION_ROUTE, FaultMode::Hang);
    }
    apis[2].set(PARTIAL_EXPIRATION_ROUTE, FaultMode::Error(500));

    let (expiration_date, _) = ecash.expiration_date_signatures();
    let mut live = 0u64;
    for (i, api) in apis.iter().enumerate() {
        let url = url::Url::parse(api.url()).unwrap();
        let client = nym_http_api_client::Client::new(url, Some(Duration::from_secs(2)));
        let start = Instant::now();
        let outcome = client
            .partial_expiration_date_signatures(Some(expiration_date), Some(EPOCH_ID))
            .await;
        let latency = start.elapsed();
        match outcome {
            Ok(partials) => {
                live += 1;
                eprintln!(
                    "probe: signer {i} served {} partial signatures in {latency:?}",
                    partials.signatures.len()
                );
            }
            Err(e) => eprintln!("probe: signer {i} failed in {latency:?}: {e}"),
        }
    }
    assert!(
        live >= ecash.threshold(),
        "with K >= threshold live signers, sufficient partials must be \
         retrievable for client-side aggregation (live: {live}, threshold: {})",
        ecash.threshold()
    );
}

/// The K < threshold side of the probe: with too few live partial-serving
/// signers, the probe correctly reports insufficiency — client-side
/// aggregation could NOT recover here, and no client-side change would help.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn threshold_probe_reports_insufficiency_below_threshold() {
    let ecash = TestEcash::new();
    let apis = [
        MockNymApi::spawn(healthy_routes(&ecash)).await,
        MockNymApi::spawn(healthy_routes(&ecash)).await,
        MockNymApi::spawn(healthy_routes(&ecash)).await,
    ];
    for api in &apis {
        api.set(AGG_EXPIRATION_ROUTE, FaultMode::Hang);
    }
    // Only ONE signer serves partials: K = 1 < threshold = 2.
    apis[1].set(PARTIAL_EXPIRATION_ROUTE, FaultMode::Error(502));
    apis[2].set(PARTIAL_EXPIRATION_ROUTE, FaultMode::Error(500));

    let (expiration_date, _) = ecash.expiration_date_signatures();
    let mut live = 0u64;
    for api in &apis {
        let url = url::Url::parse(api.url()).unwrap();
        let client = nym_http_api_client::Client::new(url, Some(Duration::from_secs(2)));
        if client
            .partial_expiration_date_signatures(Some(expiration_date), Some(EPOCH_ID))
            .await
            .is_ok()
        {
            live += 1;
        }
    }
    assert!(
        live < ecash.threshold(),
        "probe must report insufficiency below threshold (live: {live})"
    );
    eprintln!(
        "probe: {live} live partial signer(s) < threshold {} — aggregation impossible",
        ecash.threshold()
    );
}
