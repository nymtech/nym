// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use nym_network_defaults::NymContracts;
use std::str::FromStr;

// TODO: all of those could/should be derived via a macro

mod coconut_bandwidth_query_client;
mod dkg_query_client;
mod group_query_client;
mod mixnet_query_client;
mod multisig_query_client;
mod name_service_query_client;
mod sp_directory_query_client;
mod vesting_query_client;

// #[cfg(feature = "signing")]
// mod coconut_bandwidth_signing_client;
// #[cfg(feature = "signing")]
// mod dkg_signing_client;
// #[cfg(feature = "signing")]
// mod mixnet_signing_client;
// #[cfg(feature = "signing")]
// mod multisig_signing_client;
// #[cfg(feature = "signing")]
// mod name_service_signing_client;
// #[cfg(feature = "signing")]
// mod sp_directory_signing_client;
// #[cfg(feature = "signing")]
// mod vesting_signing_client;

pub use coconut_bandwidth_query_client::CoconutBandwidthQueryClient;
pub use dkg_query_client::DkgQueryClient;
pub use group_query_client::GroupQueryClient;
pub use mixnet_query_client::MixnetQueryClient;
pub use multisig_query_client::MultisigQueryClient;
pub use name_service_query_client::NameServiceQueryClient;
pub use sp_directory_query_client::SpDirectoryQueryClient;
pub use vesting_query_client::VestingQueryClient;

// #[cfg(feature = "signing")]
// pub use coconut_bandwidth_signing_client::CoconutBandwidthSigningClient;
// #[cfg(feature = "signing")]
// pub use dkg_signing_client::DkgSigningClient;
// #[cfg(feature = "signing")]
// pub use mixnet_signing_client::MixnetSigningClient;
// #[cfg(feature = "signing")]
// pub use multisig_signing_client::MultisigSigningClient;
// #[cfg(feature = "signing")]
// pub use name_service_signing_client::NameServiceSigningClient;
// #[cfg(feature = "signing")]
// pub use sp_directory_signing_client::SpDirectorySigningClient;
// #[cfg(feature = "signing")]
// pub use vesting_signing_client::VestingSigningClient;

pub trait NymContractsProvider {
    // main
    fn mixnet_contract_address(&self) -> Option<&AccountId>;
    fn vesting_contract_address(&self) -> Option<&AccountId>;

    // coconut-related
    fn coconut_bandwidth_contract_address(&self) -> Option<&AccountId>;
    fn dkg_contract_address(&self) -> Option<&AccountId>;
    fn group_contract_address(&self) -> Option<&AccountId>;
    fn multisig_contract_address(&self) -> Option<&AccountId>;

    // SPs
    fn name_service_contract_address(&self) -> Option<&AccountId>;
    fn service_provider_contract_address(&self) -> Option<&AccountId>;
}

#[derive(Debug, Clone)]
pub struct TypedNymContracts {
    pub mixnet_contract_address: Option<AccountId>,
    pub vesting_contract_address: Option<AccountId>,

    pub coconut_bandwidth_contract_address: Option<AccountId>,
    pub group_contract_address: Option<AccountId>,
    pub multisig_contract_address: Option<AccountId>,
    pub coconut_dkg_contract_address: Option<AccountId>,

    pub service_provider_directory_contract_address: Option<AccountId>,
    pub name_service_contract_address: Option<AccountId>,
}

impl TryFrom<NymContracts> for TypedNymContracts {
    type Error = <AccountId as FromStr>::Err;

    fn try_from(value: NymContracts) -> Result<Self, Self::Error> {
        Ok(TypedNymContracts {
            mixnet_contract_address: value
                .mixnet_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            vesting_contract_address: value
                .vesting_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            coconut_bandwidth_contract_address: value
                .coconut_bandwidth_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            group_contract_address: value
                .group_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            multisig_contract_address: value
                .multisig_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            coconut_dkg_contract_address: value
                .coconut_dkg_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            service_provider_directory_contract_address: value
                .service_provider_directory_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
            name_service_contract_address: value
                .name_service_contract_address
                .map(|addr| addr.parse())
                .transpose()?,
        })
    }
}

// a simple helper macro to define to repeatedly call a paged query until a full response is constructed
#[macro_export]
macro_rules! collect_paged {
    ( $self:ident, $f: ident, $field: ident ) => {{
        let mut res = Vec::new();
        let mut start_after = None;
        loop {
            let paged_response = $self.$f(start_after.take(), None).await?;
            res.extend(paged_response.$field);

            if let Some(start_next_after) = paged_response.start_next_after {
                start_after = Some(start_next_after)
            } else {
                break Ok(res);
            }
        }
    }};
}
