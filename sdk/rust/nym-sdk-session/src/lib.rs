// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! # nym-sdk-session
//!
//! A provisioning facade over `nym-registration-client`,
//! `nym-bandwidth-controller`, and the credential store. From a caller-supplied
//! mnemonic it deposits NYM and issues + persists zk-nym WireGuard ticketbooks,
//! selects gateways (by identity / two-letter country code / random), and
//! registers them, returning the per-hop WireGuard configuration the dVPN
//! datapath needs. Shared by both mixnet and dvpn modes.
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use nym_sdk_session::{GatewaySpec, Session, SessionConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! let config = SessionConfig {
//!     mnemonic: "..".parse()?,
//!     network: nym_network_defaults::NymNetworkDetails::new_mainnet(),
//!     credential_store_path: None,
//!     data_path: "/tmp/dvpn".into(),
//!     dvpn_directory_url: None,
//! };
//! let session = Session::new(config, CancellationToken::new()).await?;
//!
//! // Two-hop: entry in Germany, random exit.
//! let reg = session
//!     .register_two_hop(&GatewaySpec::Country("DE".into()), &GatewaySpec::Random)
//!     .await?;
//! # let _ = reg; Ok(())
//! # }
//! ```

mod dvpn;
mod error;
mod gateway;
mod session;

pub use dvpn::QuicBridge;
pub use error::SessionError;
pub use gateway::{GatewayInfo, GatewaySpec, SelectedGateway, WgRole};
pub use session::{HopConfig, Registration, Session, SessionConfig};
