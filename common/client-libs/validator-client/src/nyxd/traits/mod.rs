// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: expose query-related capabilities to wasm client...

mod coconut_bandwidth_query_client;
mod dkg_query_client;
mod ephemera_query_client;
mod ephemera_signing_client;
mod group_query_client;
mod mixnet_query_client;
mod multisig_query_client;
mod vesting_query_client;

mod coconut_bandwidth_signing_client;
mod dkg_signing_client;
mod mixnet_signing_client;
mod multisig_signing_client;
mod vesting_signing_client;

mod sp_directory_query_client;
mod sp_directory_signing_client;

mod name_service_query_client;
mod name_service_signing_client;

pub use coconut_bandwidth_query_client::CoconutBandwidthQueryClient;
pub use dkg_query_client::DkgQueryClient;
pub use ephemera_query_client::EphemeraQueryClient;
pub use group_query_client::GroupQueryClient;
pub use mixnet_query_client::MixnetQueryClient;
pub use multisig_query_client::MultisigQueryClient;
pub use name_service_query_client::NameServiceQueryClient;
pub use sp_directory_query_client::SpDirectoryQueryClient;
pub use vesting_query_client::VestingQueryClient;

pub use coconut_bandwidth_signing_client::CoconutBandwidthSigningClient;
pub use dkg_signing_client::DkgSigningClient;
pub use ephemera_signing_client::EphemeraSigningClient;
pub use mixnet_signing_client::MixnetSigningClient;
pub use multisig_signing_client::MultisigSigningClient;
pub use name_service_signing_client::NameServiceSigningClient;
pub use sp_directory_signing_client::SpDirectorySigningClient;
pub use vesting_signing_client::VestingSigningClient;
