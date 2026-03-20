// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use time::OffsetDateTime;

/// Discriminator for the type of node targeted by a [`TestRun`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub(crate) enum TestType {
    #[default]
    Mixnode,
    Gateway,
}

/// The data required to insert a new row into `testrun`. Does not carry an `id` since that
/// is assigned by the database on insertion.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NewTestRun {
    pub(crate) test_type: TestType,
    pub(crate) test_timestamp: OffsetDateTime,

    /// Noise handshake duration on the ingress (responder) side, in microseconds.
    pub(crate) ingress_noise_handshake_us: Option<i64>,

    /// Noise handshake duration on the egress (initiator) side, in microseconds.
    pub(crate) egress_noise_handshake_us: Option<i64>,

    pub(crate) packets_sent: i64,
    pub(crate) packets_received: i64,

    /// RTT of the initial probe packet in microseconds. `None` if the probe did not complete.
    pub(crate) approximate_latency_us: Option<i64>,

    // RTT distribution over received packets (all NULL when no packets were received).
    pub(crate) packets_rtt_min_us: Option<i64>,
    pub(crate) packets_rtt_mean_us: Option<i64>,
    pub(crate) packets_rtt_max_us: Option<i64>,
    pub(crate) packets_rtt_std_dev_us: Option<i64>,

    // Batch send latency distribution (all NULL when no batches were sent).
    pub(crate) sending_latency_min_us: Option<i64>,
    pub(crate) sending_latency_mean_us: Option<i64>,
    pub(crate) sending_latency_max_us: Option<i64>,
    pub(crate) sending_latency_std_dev_us: Option<i64>,

    pub(crate) received_duplicates: bool,

    /// First error that caused the test to abort. `None` if the run completed without error.
    pub(crate) error: Option<String>,
}

/// A row from the `testrun` table, as returned by a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRun {
    pub(crate) id: i64,

    #[sqlx(flatten)]
    pub(crate) inner: NewTestRun,
}

/// The data required to insert or update a row in `nym_node`. Does not carry `last_testrun`
/// since that is managed separately via [`StorageManager::set_node_last_testrun`].
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NewNymNode {
    /// Node ID as assigned by the mixnet contract.
    pub(crate) node_id: i64,

    /// Ed25519 identity key, base58-encoded.
    /// A node_id always maps to exactly one identity_key and is never reassigned.
    pub(crate) identity_key: String,

    /// When this node was last observed as bonded in the contract.
    pub(crate) last_seen_bonded: OffsetDateTime,

    /// Mixnet socket address (host:port) at which the node accepts sphinx packets.
    /// Stored as a string; parse with `str::parse::<SocketAddr>()` when needed.
    pub(crate) mixnet_socket_address: String,

    /// X25519 public key used for Noise handshakes, base58-encoded.
    /// `None` if retrieval from the node failed.
    pub(crate) noise_key: Option<String>,

    /// Sphinx public key used for packet encryption, base58-encoded.
    /// `None` if retrieval from the node failed.
    /// Always `None`/`Some` together with `key_rotation_id`.
    pub(crate) sphinx_key: Option<String>,

    /// Key rotation epoch ID that `sphinx_key` belongs to.
    /// `None` if retrieval from the node failed.
    /// Always `None`/`Some` together with `sphinx_key`.
    pub(crate) key_rotation_id: Option<i64>,
}

/// A row from the `testrun_in_progress` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRunInProgress {
    pub(crate) node_id: i64,
    pub(crate) started_at: OffsetDateTime,
}

/// A row from the `nym_node` table, as returned by a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NymNode {
    #[sqlx(flatten)]
    pub(crate) inner: NewNymNode,

    /// ID of the most recent test run against this node. `None` if never tested.
    pub(crate) last_testrun: Option<i64>,
}
