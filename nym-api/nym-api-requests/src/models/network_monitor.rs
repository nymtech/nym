// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::pagination::PaginatedResponse;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_mixnet_contract_common::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, Default, ToSchema)]
pub struct TestNode {
    pub node_id: Option<u32>,
    pub identity_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct TestRoute {
    pub gateway: TestNode,
    pub layer1: TestNode,
    pub layer2: TestNode,
    pub layer3: TestNode,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct PartialTestResult {
    pub monitor_run_id: i64,
    pub timestamp: i64,
    pub overall_reliability_for_all_routes_in_monitor_run: Option<u8>,
    pub test_routes: TestRoute,
}

pub type MixnodeTestResultResponse = PaginatedResponse<PartialTestResult>;
pub type GatewayTestResultResponse = PaginatedResponse<PartialTestResult>;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NetworkMonitorRunDetailsResponse {
    pub monitor_run_id: i64,
    pub network_reliability: f64,
    pub total_sent: usize,
    pub total_received: usize,

    // integer score to number of nodes with that score
    pub mixnode_results: BTreeMap<u8, usize>,
    pub gateway_results: BTreeMap<u8, usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/MixnodeCoreStatusResponse.ts"
    )
)]
pub struct MixnodeCoreStatusResponse {
    pub mix_id: NodeId,
    pub count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/GatewayCoreStatusResponse.ts"
    )
)]
pub struct GatewayCoreStatusResponse {
    pub identity: String,
    pub count: i64,
}

/// Request/response types for the v3 network-monitor flow, in which an orchestrator submits
/// stress testing results to nym-api via signed batches.
pub mod v3 {
    use super::*;
    use crate::signable::SignedMessage;
    use std::time::Duration;
    use time::OffsetDateTime;

    /// Signed envelope posted by a network monitor orchestrator to
    /// `POST /v3/nym-nodes/stress-testing/batch-submit`.
    ///
    /// The signature is checked against the `signer` field of the inner
    /// [`StressTestBatchSubmissionContent`], which must also match one of the orchestrators
    /// registered in the network-monitors contract.
    pub type StressTestBatchSubmission = SignedMessage<StressTestBatchSubmissionContent>;

    /// Confirmation returned to an orchestrator after a successful submission, reporting what
    /// became of the individual results. The three counts sum to the number of results submitted.
    ///
    /// Reporting these matters because an accepted batch can still store nothing: rows deduplicate
    /// at the database with insert-or-ignore semantics, so without a count the submitter cannot
    /// distinguish "stored" from "silently discarded" - both are a `200`.
    ///
    /// Every field is optional so that a newer orchestrator can still read the empty body returned
    /// by a nym-api predating these counts. `None` therefore means "not reported", which is a
    /// different signal from `Some(0)`.
    #[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
    pub struct StressTestBatchSubmissionResponse {
        /// Results newly stored by this submission.
        #[serde(default)]
        pub accepted: Option<usize>,

        /// Results that were already stored, i.e. this submission re-sent a measurement nym-api had
        /// seen before. Expected to be non-zero on the orchestrator's at-least-once retry path; a
        /// persistently non-zero value means measurements are being discarded.
        #[serde(default)]
        pub duplicates: Option<usize>,

        /// Results dropped by per-entry validation (a non-mixnode entry, or a performance score
        /// outside `[0.0, 1.0]`).
        #[serde(default)]
        pub rejected: Option<usize>,
    }

    /// Single stress-test measurement for one node, produced by a network monitor orchestrator.
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct StressTestResult {
        /// Orchestrator-local id of the test run that produced this result. Combined with the
        /// batch's `signer` it uniquely identifies the measurement, allowing nym-api to dedupe
        /// retried submissions on the at-least-once delivery path.
        pub testrun_id: i64,

        /// Contract-assigned id of the node that was tested.
        pub node_id: NodeId,

        /// Whether the tested node was acting as a mixnode during the measurement.
        ///
        /// Included explicitly (rather than inferred from on-chain role) so the API can reject or
        /// route entries that don't match the expected role without re-querying the contract.
        pub is_mixnode: bool,

        #[schema(value_type = String)]
        #[serde(with = "time::serde::rfc3339")]
        pub test_timestamp: OffsetDateTime,

        /// Measured performance score in the `[0.0, 1.0]` range.
        pub test_performance: f64,

        /// Whether the node responded at all during testing.
        ///
        /// Recorded alongside `test_performance` so that a genuine 0.0 score (node responded but
        /// dropped everything) can be distinguished from the node being offline entirely.
        pub was_reachable: bool,
    }

    /// Body of a stress-test batch submission, signed by a network monitor orchestrator.
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct StressTestBatchSubmissionContent {
        /// ed25519 identity key of the submitting orchestrator. Must match an entry in the
        /// network-monitors contract for the batch to be accepted.
        #[schema(value_type = String)]
        #[serde(with = "ed25519::bs58_ed25519_pubkey")]
        pub signer: ed25519::PublicKey,

        /// Time at which this batch was produced. Also used as a monotonic nonce for replay
        /// protection: the API rejects submissions whose timestamp is not strictly greater than
        /// the orchestrator's previous accepted submission.
        #[schema(value_type = String)]
        #[serde(with = "time::serde::rfc3339")]
        pub timestamp: OffsetDateTime,

        pub results: Vec<StressTestResult>,
    }

    impl StressTestBatchSubmissionContent {
        /// Build a batch submission body stamped with the current UTC time.
        pub fn new(signer: ed25519::PublicKey, results: Vec<StressTestResult>) -> Self {
            StressTestBatchSubmissionContent {
                signer,
                timestamp: OffsetDateTime::now_utc(),
                results,
            }
        }

        /// Whether this submission is older than `max_age` relative to the current UTC time.
        ///
        /// Used server-side to reject submissions that have been sitting around too long, even if
        /// they are otherwise well-formed and correctly signed.
        pub fn is_stale(&self, max_age: Duration) -> bool {
            self.timestamp + max_age < OffsetDateTime::now_utc()
        }
    }

    /// Response body for `GET /v3/nym-nodes/stress-testing/known-monitors/{identity_key}`,
    /// used by orchestrators to check whether this nym-api currently recognises their key.
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct KnownNetworkMonitorResponse {
        /// The ed25519 identity key that was queried (base58-encoded on the wire).
        #[serde(with = "bs58_ed25519_pubkey")]
        #[schema(value_type = String)]
        pub identity_key: ed25519::PublicKey,

        /// Whether the queried identity key is currently recognised by this nym-api
        /// as an authorised network monitor permitted to submit stress testing results.
        pub authorised: bool,
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::signable::SignableMessageBody;
        use nym_test_utils::helpers::deterministic_rng;
        use time::macros::datetime;

        fn dummy_results() -> Vec<StressTestResult> {
            // Order-distinguishable entries: if deserialisation ever permuted the array, the
            // re-serialised body would no longer match the signed bytes, and `verify_signature`
            // would return false. `testrun_id` is the order witness.
            vec![
                StressTestResult {
                    testrun_id: 1,
                    node_id: 42,
                    is_mixnode: true,
                    test_timestamp: datetime!(2026-06-01 12:34:56.123456789 UTC),
                    test_performance: 0.6666666666666666,
                    was_reachable: true,
                },
                StressTestResult {
                    testrun_id: 2,
                    node_id: 7,
                    is_mixnode: true,
                    test_timestamp: datetime!(2026-06-01 12:34:56 UTC),
                    test_performance: 0.0,
                    was_reachable: false,
                },
                StressTestResult {
                    testrun_id: 3,
                    node_id: u32::MAX,
                    is_mixnode: true,
                    test_timestamp: datetime!(2026-06-01 12:34:56.999999999 UTC),
                    test_performance: 1.0,
                    was_reachable: true,
                },
            ]
        }

        // Integrity check on the wire is `serde_json::to_vec(deserialize(serde_json::to_vec(body)))
        // == serde_json::to_vec(body)`. If JSON serialisation isn't a fixed point, every batch
        // submission would fail nym-api's signature verification. Cover the timestamp shapes the
        // orchestrator actually produces, including the `+1ns` bump from the monotonicity safeguard.
        #[test]
        fn signed_batch_submission_roundtrips_through_json() {
            let mut rng = deterministic_rng();
            let keys = ed25519::KeyPair::new(&mut rng);

            let timestamps = [
                datetime!(2026-06-01 12:34:56 UTC),
                datetime!(2026-06-01 12:34:56.000000001 UTC),
                datetime!(2026-06-01 12:34:56.999999999 UTC),
                datetime!(2026-06-01 12:34:56.123456789 UTC),
                OffsetDateTime::now_utc(),
                OffsetDateTime::now_utc() + time::Duration::NANOSECOND,
            ];

            for timestamp in timestamps {
                let body = StressTestBatchSubmissionContent {
                    signer: *keys.public_key(),
                    timestamp,
                    results: dummy_results(),
                };
                let signed = body.clone().sign(keys.private_key());

                let bytes = serde_json::to_vec(&signed).unwrap();
                let deserialised: StressTestBatchSubmission =
                    serde_json::from_slice(&bytes).unwrap();

                // The handler verifies against `body.body.signer` — match that exactly.
                assert!(
                    deserialised.verify_signature(&deserialised.body.signer),
                    "signature failed to verify after JSON round-trip for timestamp {timestamp}",
                );
                assert_eq!(deserialised.body.timestamp, timestamp);
            }
        }

        // Every f64 that the orchestrator's `received as f64 / sent as f64` formula can produce
        // (storage/models.rs) must round-trip byte-exactly through JSON. Exhaustively cover the
        // range and exercise sent values that produce non-terminating fractions (1/3, 1/7, ...).
        #[test]
        fn computed_test_performance_values_roundtrip() {
            for sent in 1u64..=200 {
                for received in 0u64..=(sent * 2) {
                    let perf = received as f64 / sent as f64;
                    let s = serde_json::to_string(&perf).unwrap();
                    let perf2: f64 = serde_json::from_str(&s).unwrap();
                    let s2 = serde_json::to_string(&perf2).unwrap();
                    assert_eq!(
                        s, s2,
                        "f64 round-trip mismatch for {received}/{sent} = {perf}: {s} -> {s2}",
                    );
                }
            }
        }

        // serde_json serialises non-finite f64 as `null`. Confirm what the deserialiser does with
        // `null` for a struct field typed as f64 - if it succeeds with a default value (rather than
        // erroring), a NaN/Infinity test_performance could silently break signature verification
        // because the re-serialised body would no longer have `null` at that position.
        #[test]
        fn non_finite_test_performance_breaks_loudly_not_silently() {
            let nan_result = StressTestResult {
                testrun_id: 1,
                node_id: 1,
                is_mixnode: true,
                test_timestamp: datetime!(2026-06-01 12:34:56 UTC),
                test_performance: f64::NAN,
                was_reachable: true,
            };
            let json = serde_json::to_string(&nan_result).unwrap();
            // NaN serialises as `null` - this is the dangerous shape
            assert!(
                json.contains(r#""test_performance":null"#),
                "expected NaN to serialise as null: {json}",
            );
            // ...and `null` MUST fail to deserialise rather than silently becoming 0.0 / default;
            // if this ever changes, NaN would silently corrupt signature verification.
            let deserialised: Result<StressTestResult, _> = serde_json::from_str(&json);
            assert!(
                deserialised.is_err(),
                "deserialising null into f64 unexpectedly succeeded - signature verification \
                 would silently fail for any submission containing a non-finite test_performance",
            );
        }

        // Specifically pin the two hypotheses we want to rule out:
        //   1. Vec<StressTestResult> serialisation/deserialisation preserves order.
        //   2. The body bytes serialised standalone (= what gets signed) are byte-identical to
        //      the body sub-object bytes embedded in the outer SignedMessage JSON (= what the
        //      server sees after parsing). Re-serialising the deserialised body must reproduce
        //      the signed bytes verbatim, otherwise no signature could ever verify.
        #[test]
        fn batch_body_serialisation_is_a_byte_exact_fixed_point() {
            let mut rng = deterministic_rng();
            let keys = ed25519::KeyPair::new(&mut rng);

            let body = StressTestBatchSubmissionContent {
                signer: *keys.public_key(),
                timestamp: datetime!(2026-06-01 12:34:56.123456789 UTC),
                results: dummy_results(),
            };

            let signed_bytes = body.plaintext();
            let body_str = std::str::from_utf8(&signed_bytes).unwrap();

            // (1) array order preserved on the wire
            let pos1 = body_str.find(r#""testrun_id":1"#).unwrap();
            let pos2 = body_str.find(r#""testrun_id":2"#).unwrap();
            let pos3 = body_str.find(r#""testrun_id":3"#).unwrap();
            assert!(pos1 < pos2 && pos2 < pos3, "JSON: {body_str}");

            // (2) round-trip is byte-exact
            let deserialised: StressTestBatchSubmissionContent =
                serde_json::from_slice(&signed_bytes).unwrap();
            let resigned_bytes = deserialised.plaintext();
            assert_eq!(
                signed_bytes, resigned_bytes,
                "deserialise-then-re-serialise was not a fixed point"
            );
        }

        // nym-api and the orchestrator are deployed independently, so an orchestrator carrying the
        // submission counts may talk to a nym-api that predates them and answers a bare `{}`. That
        // must deserialise rather than error - a hard failure here would break submissions outright,
        // which is worse than the missing telemetry - and it must land as `None` rather than
        // `Some(0)`, because the orchestrator warns on a non-zero duplicate count and "not reported"
        // must not be mistaken for "nothing was stored".
        #[test]
        fn submission_response_tolerates_a_body_without_counts() {
            let old: StressTestBatchSubmissionResponse = serde_json::from_str("{}")
                .expect("a response body predating the counts must still deserialise");
            assert_eq!(old.accepted, None);
            assert_eq!(old.duplicates, None);
            assert_eq!(old.rejected, None);

            // and a populated body round-trips with its counts intact
            let reported = StressTestBatchSubmissionResponse {
                accepted: Some(48),
                duplicates: Some(1),
                rejected: Some(1),
            };
            let json = serde_json::to_string(&reported).unwrap();
            let parsed: StressTestBatchSubmissionResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.accepted, Some(48));
            assert_eq!(parsed.duplicates, Some(1));
            assert_eq!(parsed.rejected, Some(1));
        }

        // The mirror of the above: a nym-api reporting the counts, answering an orchestrator that
        // predates them and whose type carried no fields at all. Serde must ignore the unknown keys
        // rather than error.
        //
        // This is the more dangerous of the two directions. The client parses with a plain
        // `serde_json::from_slice`, so a decode failure surfaces as a failed POST, and the
        // submission watermark is only advanced after a POST succeeds - an old orchestrator would
        // therefore treat every batch as failed, resubmit the same rows forever and never make
        // forward progress, while nym-api quietly stored them on the first attempt.
        #[test]
        fn submission_response_counts_are_ignored_by_a_reader_predating_them() {
            // faithful copy of the previously deployed shape
            #[derive(Deserialize)]
            struct OldStressTestBatchSubmissionResponse {}

            let json = serde_json::to_string(&StressTestBatchSubmissionResponse {
                accepted: Some(48),
                duplicates: Some(1),
                rejected: Some(1),
            })
            .unwrap();

            let parsed: Result<OldStressTestBatchSubmissionResponse, _> =
                serde_json::from_str(&json);
            assert!(
                parsed.is_ok(),
                "a reader predating the counts rejected {json}: {:?}",
                parsed.err(),
            );
        }
    }
}
