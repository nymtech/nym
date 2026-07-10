// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session configuration: [`SessionConfig`] and the opt-in [`RestockPolicy`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use nym_bandwidth_controller::config::BandwidthControllerConfig;
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_network_defaults::NymNetworkDetails;

/// Opt-in policy for automatic chain-side ticketbook restock. Maps onto the bandwidth controller's
/// restock thresholds. Only in effect when set via [`SessionConfig::automatic_topups`].
#[derive(Clone, Copy, Debug)]
pub struct RestockPolicy {
    /// Restock a ticket type once its usable stock drops to/below this many tickets.
    pub restock_below_tickets: u64,
    /// Minimum usable tickets for a type to be considered "ready to connect".
    pub readiness_min_tickets: u64,
    /// How often to proactively check stock.
    pub check_interval: Duration,
    /// Treat a ticketbook expiring within this window as needing replacement.
    pub soon_expiry: Duration,
}

impl Default for RestockPolicy {
    fn default() -> Self {
        // Mirror `BandwidthControllerConfig::default()`.
        Self {
            restock_below_tickets: 20,
            readiness_min_tickets: 5,
            check_interval: Duration::from_secs(3 * 3600),
            soon_expiry: Duration::from_secs(12 * 3600),
        }
    }
}

impl From<RestockPolicy> for BandwidthControllerConfig {
    fn from(p: RestockPolicy) -> Self {
        BandwidthControllerConfig {
            topup_interval: p.check_interval,
            soon_expiry_threshold: p.soon_expiry,
            nb_ticket_restock: p.restock_below_tickets,
            min_nb_ticket_needed: p.readiness_min_tickets,
            // The session scopes this to its WireGuard types when installing the config; the
            // default is only a placeholder.
            ..Default::default()
        }
    }
}

/// Configuration for creating a [`Session`].
pub struct SessionConfig {
    /// Funded chain mnemonic used to deposit NYM and issue ticketbooks. Ignored when
    /// [`bandwidth_provider`](Self::bandwidth_provider) is set.
    pub mnemonic: bip39::Mnemonic,
    /// Network to operate against (contract addresses, endpoints, denoms).
    pub network: NymNetworkDetails,
    /// Persistent credential store path. `None` uses a file under `data_path`
    /// (a fully ephemeral in-memory store is not used so tickets survive a
    /// bring-down/bring-up cycle).
    pub credential_store_path: Option<PathBuf>,
    /// Directory for the fetcher's pending-request recovery database and other
    /// per-session data.
    pub data_path: PathBuf,
    /// Optional dVPN gateway-directory URL. When set, the session fetches it to
    /// enrich gateway monikers and to enable QUIC-bridge entry selection
    /// (`register_two_hop_quic`). Fetched best-effort — a failure is logged and
    /// treated as an empty directory.
    pub dvpn_directory_url: Option<String>,
    /// Opt-in automatic chain-side restock. `None` (default) provisions once and never deposits in
    /// the background; the tunnel still tops up from already-stored tickets. `Some(policy)` lets a
    /// long-lived session re-issue ticketbooks when stock runs low (this spends NYM).
    pub automatic_topups: Option<RestockPolicy>,
    /// Externally-managed bandwidth provider. When set, the session uses it for all ticket
    /// spending and does NOT spawn its own controller — for callers already running a controller
    /// over the same credential store (preserving the single-writer invariant). `mnemonic` and the
    /// credential store are then unused, and the caller is responsible for provisioning.
    pub bandwidth_provider: Option<Arc<dyn BandwidthTicketProvider>>,
    /// Reuse persisted gateway registrations (default: `true`). A successful registration is
    /// stored under `data_path` (client WireGuard key + assigned configuration) and served back
    /// on later registrations against the same gateway/role — no gateway exchange, no ticket
    /// spent, resuming the peer's remaining bandwidth allowance. Validate reused registrations
    /// by use (`Tunnel::await_established`) and fall back via
    /// [`crate::Session::invalidate_registration`].
    ///
    /// Privacy trade-off: reuse links this client's connections to the same WireGuard peer
    /// identity at the gateway across sessions. Set to `false` for an unlinkable fresh peer per
    /// connection — every registration then spends a ticket, and nothing is persisted.
    pub reuse_registrations: bool,
}

impl SessionConfig {
    /// A config with the required fields and sensible defaults (no automatic topups, own controller).
    pub fn new(mnemonic: bip39::Mnemonic, network: NymNetworkDetails, data_path: PathBuf) -> Self {
        Self {
            mnemonic,
            network,
            credential_store_path: None,
            data_path,
            dvpn_directory_url: None,
            automatic_topups: None,
            bandwidth_provider: None,
            reuse_registrations: true,
        }
    }

    /// Opt into automatic chain-side restock with the given policy (this can spend NYM).
    #[must_use]
    pub fn with_automatic_topups(mut self, policy: RestockPolicy) -> Self {
        self.automatic_topups = Some(policy);
        self
    }

    /// Use an externally-managed bandwidth provider instead of spawning an own controller.
    #[must_use]
    pub fn with_bandwidth_provider(mut self, provider: Arc<dyn BandwidthTicketProvider>) -> Self {
        self.bandwidth_provider = Some(provider);
        self
    }

    /// Set the dVPN directory URL.
    #[must_use]
    pub fn with_dvpn_directory_url(mut self, url: impl Into<String>) -> Self {
        self.dvpn_directory_url = Some(url.into());
        self
    }

    /// Set the credential store path.
    #[must_use]
    pub fn with_credential_store_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.credential_store_path = Some(path.into());
        self
    }
}
