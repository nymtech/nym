// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use nym_network_defaults::NymContracts;
use std::str::FromStr;

// TODO: all of those could/should be derived via a macro

// query clients
pub mod dkg_query_client;
pub mod ecash_query_client;
pub mod group_query_client;
pub mod mixnet_query_client;
pub mod multisig_query_client;
pub mod vesting_query_client;

// signing clients
pub mod dkg_signing_client;
pub mod ecash_signing_client;
pub mod group_signing_client;
pub mod mixnet_signing_client;
pub mod multisig_signing_client;
pub mod vesting_signing_client;

// re-export query traits
pub use dkg_query_client::{DkgQueryClient, PagedDkgQueryClient};
pub use ecash_query_client::{EcashQueryClient, PagedEcashQueryClient};
pub use group_query_client::{GroupQueryClient, PagedGroupQueryClient};
pub use mixnet_query_client::{MixnetQueryClient, PagedMixnetQueryClient};
pub use multisig_query_client::{MultisigQueryClient, PagedMultisigQueryClient};
pub use vesting_query_client::{PagedVestingQueryClient, VestingQueryClient};

// re-export signing traits
pub use dkg_signing_client::DkgSigningClient;
pub use ecash_signing_client::EcashSigningClient;
pub use group_signing_client::GroupSigningClient;
pub use mixnet_signing_client::MixnetSigningClient;
pub use multisig_signing_client::MultisigSigningClient;
pub use vesting_signing_client::VestingSigningClient;

// helper for providing blanket implementation for query clients
pub trait NymContractsProvider {
    // main
    fn mixnet_contract_address(&self) -> Option<&AccountId>;
    fn vesting_contract_address(&self) -> Option<&AccountId>;

    // coconut-related
    fn coconut_bandwidth_contract_address(&self) -> Option<&AccountId>;
    fn dkg_contract_address(&self) -> Option<&AccountId>;
    fn group_contract_address(&self) -> Option<&AccountId>;
    fn multisig_contract_address(&self) -> Option<&AccountId>;
}

#[derive(Debug, Clone)]
pub struct TypedNymContracts {
    pub mixnet_contract_address: Option<AccountId>,
    pub vesting_contract_address: Option<AccountId>,

    pub coconut_bandwidth_contract_address: Option<AccountId>,
    pub group_contract_address: Option<AccountId>,
    pub multisig_contract_address: Option<AccountId>,
    pub coconut_dkg_contract_address: Option<AccountId>,
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
        })
    }
}

// a simple helper macro to define to repeatedly call a paged query until a full response is constructed
#[macro_export]
macro_rules! collect_paged {
    // TODO: deal with the args in a nicer way
    ( $self:ident, $f: ident, $field: ident ) => {{
        let mut res = Vec::new();
        let mut start_after = None;
        loop {
            let paged_response = $self.$f(start_after.take(), None).await?;
            res.extend(paged_response.$field);

            if let Some(start_next_after) = paged_response.start_next_after {
                start_after = Some(start_next_after.into())
            } else {
                break Ok(res);
            }
        }
    }};

    ( $self:ident, $f: ident, $field: ident, $($args:tt),*) => {{
        let mut res = Vec::new();
        let mut start_after = None;
        loop {
            let paged_response = $self.$f($($args),*, start_after.take(), None).await?;
            res.extend(paged_response.$field);

            if let Some(start_next_after) = paged_response.start_next_after {
                start_after = Some(start_next_after.into())
            } else {
                break Ok(res);
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use crate::nyxd::Coin;

    pub(crate) trait IgnoreValue {
        fn ignore(self) -> u32
        where
            Self: Sized,
        {
            42
            // reason we're returning a value as opposed to just `()` is that whenever we match on all enums
            // we don't want to accidentally miss a variant because compiler will treat it the same way
        }
    }

    impl<T> IgnoreValue for T {}

    pub(crate) fn mock_coin() -> Coin {
        Coin::new(42, "ufoomp")
    }
}
