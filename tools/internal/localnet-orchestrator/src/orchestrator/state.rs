// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Default, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum LocalnetState {
    /// Defines brand new network without anything deployed on it
    #[default]
    Uninitialised,

    /// Defines network that only has a nyxd instance on it
    RunningNyxd,

    /// Defines network that has had cosmwasm smart contracts initialised on it
    DeployedNymContracts,

    /// Defines network with a functional instance of nym-api that is capable of issuing zk-nyms
    RunningNymApi,

    /// Defines network with a functional mixnet
    // TODO: might have to split between running and bonding
    RunningNymNodes, // more steps could be added later to indicate, for example, deployed credential proxy or vpn api
}
