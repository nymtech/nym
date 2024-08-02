// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Delegation, EpochId, MixId, MixNodeCostParams, MixNodeRewarding};
use cosmwasm_std::{Addr, Coin};
use std::collections::HashMap;

use crate::error::MixnetContractError;
use crate::rewarding::helpers::truncate_reward;

pub struct SimulatedNode {
    pub mix_id: MixId,
    pub rewarding_details: MixNodeRewarding,
    pub delegations: HashMap<String, Delegation>,
}

impl SimulatedNode {
    pub fn new(
        mix_id: MixId,
        cost_params: MixNodeCostParams,
        initial_pledge: &Coin,
        current_epoch: EpochId,
    ) -> Result<Self, MixnetContractError> {
        Ok(SimulatedNode {
            mix_id,
            rewarding_details: MixNodeRewarding::initialise_new(
                cost_params,
                initial_pledge,
                current_epoch,
            )?,
            delegations: HashMap::new(),
        })
    }

    pub fn delegate<S: Into<String>>(
        &mut self,
        delegator: S,
        delegation: Coin,
    ) -> Result<(), MixnetContractError> {
        self.rewarding_details
            .add_base_delegation(delegation.amount)?;

        let delegator = delegator.into();
        let delegation = Delegation::new(
            Addr::unchecked(&delegator),
            self.mix_id,
            self.rewarding_details.total_unit_reward,
            delegation,
            42,
        );

        self.delegations.insert(delegator, delegation);
        Ok(())
    }

    pub fn undelegate<S: Into<String>>(
        &mut self,
        delegator: S,
    ) -> Result<(Coin, Coin), MixnetContractError> {
        let delegator = delegator.into();
        let delegation = self.delegations.remove(&delegator).ok_or(
            MixnetContractError::NoMixnodeDelegationFound {
                mix_id: MixId::MAX,
                address: delegator,
                proxy: None,
            },
        )?;

        let reward = self
            .rewarding_details
            .determine_delegation_reward(&delegation)?;
        self.rewarding_details
            .remove_delegation_decimal(delegation.dec_amount()? + reward)?;

        let reward_denom = &delegation.amount.denom;
        let truncated_reward = truncate_reward(reward, reward_denom);

        Ok((delegation.amount, truncated_reward))
    }
}
