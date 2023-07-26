// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: expose query-related capabilities to wasm client...

mod coconut_bandwidth_query_client;
mod dkg_query_client;
mod group_query_client;
mod mixnet_query_client;
mod multisig_query_client;
mod name_service_query_client;
mod sp_directory_query_client;
mod vesting_query_client;

#[cfg(feature = "signing")]
mod coconut_bandwidth_signing_client;
#[cfg(feature = "signing")]
mod dkg_signing_client;
#[cfg(feature = "signing")]
mod mixnet_signing_client;
#[cfg(feature = "signing")]
mod multisig_signing_client;
#[cfg(feature = "signing")]
mod name_service_signing_client;
#[cfg(feature = "signing")]
mod sp_directory_signing_client;
#[cfg(feature = "signing")]
mod vesting_signing_client;

pub use coconut_bandwidth_query_client::CoconutBandwidthQueryClient;
pub use dkg_query_client::DkgQueryClient;
pub use group_query_client::GroupQueryClient;
pub use mixnet_query_client::MixnetQueryClient;
pub use multisig_query_client::MultisigQueryClient;
pub use name_service_query_client::NameServiceQueryClient;
pub use sp_directory_query_client::SpDirectoryQueryClient;
pub use vesting_query_client::VestingQueryClient;

#[cfg(feature = "signing")]
pub use coconut_bandwidth_signing_client::CoconutBandwidthSigningClient;
#[cfg(feature = "signing")]
pub use dkg_signing_client::DkgSigningClient;
#[cfg(feature = "signing")]
pub use mixnet_signing_client::MixnetSigningClient;
#[cfg(feature = "signing")]
pub use multisig_signing_client::MultisigSigningClient;
#[cfg(feature = "signing")]
pub use name_service_signing_client::NameServiceSigningClient;
#[cfg(feature = "signing")]
pub use sp_directory_signing_client::SpDirectorySigningClient;
#[cfg(feature = "signing")]
pub use vesting_signing_client::VestingSigningClient;
