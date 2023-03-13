// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod bond_gateway;
pub mod gateway_bonding_sign_payload;
pub mod unbond_gateway;
pub mod vesting_bond_gateway;
pub mod vesting_unbond_gateway;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsGateway {
    #[clap(subcommand)]
    pub command: MixnetOperatorsGatewayCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsGatewayCommands {
    /// Bond to a gateway
    Bond(bond_gateway::Args),
    /// Unbond from a gateway
    Unbond(unbond_gateway::Args),
    /// Bond to a gateway with locked tokens
    VestingBond(vesting_bond_gateway::Args),
    /// Unbound from a gateway (when originally using locked tokens)
    VestingUnbound(vesting_unbond_gateway::Args),
    /// Create base58-encoded payload required for producing valid bonding signature.
    CreateGatewayBondingSignPayload(gateway_bonding_sign_payload::Args),
}
