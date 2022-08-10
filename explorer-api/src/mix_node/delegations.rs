// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::models::SummedDelegations;
use crate::client::ThreadsafeValidatorClient;
use itertools::Itertools;
use mixnet_contract_common::{Delegation, NodeId};

pub(crate) async fn get_single_mixnode_delegations(
    client: &ThreadsafeValidatorClient,
    mix_id: NodeId,
) -> Vec<Delegation> {
    match client
        .0
        .get_all_nymd_single_mixnode_delegations(mix_id)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Could not get delegations for mix node {}: {:?}", mix_id, e);
            vec![]
        }
    }
}

pub(crate) async fn get_single_mixnode_delegations_summed(
    client: &ThreadsafeValidatorClient,
    mix_id: NodeId,
) -> Vec<SummedDelegations> {
    let delegations_by_owner = get_single_mixnode_delegations(client, mix_id)
        .await
        .into_iter()
        .into_group_map_by(|delegation| delegation.owner.clone());

    delegations_by_owner
        .iter()
        .filter_map(|(_, delegations)| SummedDelegations::from(delegations))
        .collect()
}
