// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # nym-sdk-session
//!
//! A provisioning facade over `nym-registration-client`,
//! `nym-bandwidth-controller`, and the credential store. From a caller-supplied
//! mnemonic it deposits NYM and issues + persists zk-nym WireGuard ticketbooks,
//! selects gateways (by identity / two-letter country code / random), and
//! registers them, returning the per-hop WireGuard configuration the dVPN
//! datapath needs. Shared by both mixnet and dvpn modes.
//!
//! ## Registration reuse
//!
//! Registrations are persisted (client WireGuard key + assigned configuration,
//! in `registrations.json` under `data_path`) and served back on later
//! `register_*` calls against the same network/gateway/role — no gateway
//! exchange, **no ticket spent**, resuming the gateway-side peer's remaining
//! bandwidth allowance. Cached registrations are validated by use: gate tunnel
//! bring-up with `Tunnel::await_established` (in `smoldvpn`) and, when a
//! hop fails to establish, call [`Session::invalidate_registration`] for it and
//! register again (which spends a ticket and persists the fresh peer).
//! Default-on; opt out via `SessionConfig::reuse_registrations` (see its doc
//! for the privacy trade-off). The cache file holds WireGuard private keys —
//! the same secret-sensitivity class as the credential store beside it.
//!
//! ## Signer-failure tolerance
//!
//! Unresponsive ecash signers are a normal operating condition, not an error to
//! hang on. The session bounds every read-only global-signing-data fetch with a
//! per-call timeout ([`TimeoutFetcher`], [`DEFAULT_PUBLIC_DATA_TIMEOUT`]) — the
//! deposit/issuance call is deliberately exempt — and bounds provisioning
//! overall, surfacing [`SessionError::ProvisioningTimeout`] instead of blocking.
//! An issued (paid-for) ticketbook is persisted even when the signing data
//! needed to *spend* it cannot currently be fetched: retries never re-deposit,
//! spends during the outage fail fast with a clear error, and everything works
//! again the moment enough signers return.
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use nym_sdk_session::{GatewaySpec, Session, SessionConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! // Provisions once and tops up a live tunnel from stored tickets; opt into background
//! // re-issuance with `.with_automatic_topups(..)`.
//! let config = SessionConfig::new(
//!     "..".parse()?,
//!     nym_network_defaults::NymNetworkDetails::new_mainnet(),
//!     "/tmp/dvpn".into(),
//! );
//! let session = Session::new(config, CancellationToken::new()).await?;
//!
//! // Two-hop: entry in Germany, random exit.
//! let reg = session
//!     .register_two_hop(&GatewaySpec::Country("DE".into()), &GatewaySpec::Random)
//!     .await?;
//! # let _ = reg; Ok(())
//! # }
//! ```

mod config;
mod dvpn;
mod error;
mod fetcher;
mod gateway;
mod registration_cache;
mod session;

pub use config::{RestockPolicy, SessionConfig};
pub use dvpn::QuicBridge;
pub use error::SessionError;
pub use fetcher::{SignerTimeout, TimeoutFetcher, DEFAULT_PUBLIC_DATA_TIMEOUT};
pub use gateway::{GatewayInfo, GatewaySpec, SelectedGateway, WgRole};
pub use session::{HopConfig, Registration, Session};
