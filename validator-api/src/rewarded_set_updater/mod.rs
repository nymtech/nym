// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// there is couple of reasons for putting this in a separate module:
// 1. I didn't feel it fit well in validator "cache". It seems like purpose of cache is to just keep updating local data
//    rather than attempting to change global view (i.e. the active set)
//
// 2. However, even if it was to exist in the validator cache refresher, we'd have to create a different "run"
//    method as it doesn't have access to the signing client which we need in the case of updating rewarded sets
//    (because validator cache can be run by anyone regardless of whether, say, network monitor exists)
//
// 3. Eventually this whole procedure is going to get expanded to allow for distribution of rewarded set generation
//    and hence this might be a good place for it.

use crate::contract_cache::ValidatorCache;
use crate::nymd_client::Client;
use mixnet_contract_common::{IdentityKey, MixNodeBond};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::sync::Arc;
use tokio::sync::Notify;
use validator_client::nymd::SigningNymdClient;

pub struct RewardedSetUpdater {
    nymd_client: Client<SigningNymdClient>,
    update_rewarded_set_notify: Arc<Notify>,
    validator_cache: ValidatorCache,
}

impl RewardedSetUpdater {
    pub(crate) fn new(
        nymd_client: Client<SigningNymdClient>,
        update_rewarded_set_notify: Arc<Notify>,
        validator_cache: ValidatorCache,
    ) -> Self {
        RewardedSetUpdater {
            nymd_client,
            update_rewarded_set_notify,
            validator_cache,
        }
    }

    fn determine_rewarded_set(
        &self,
        mixnodes: Vec<MixNodeBond>,
        nodes_to_select: u32,
    ) -> Vec<IdentityKey> {
        if mixnodes.is_empty() {
            return Vec::new();
        }

        let mut rng = OsRng;

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .into_iter()
            .map(|mix| {
                // note that the theoretical maximum possible stake is equal to the total
                // supply of all tokens, i.e. 1B (which is 1 quadrillion of native tokens, i.e. 10^15 ~ 2^50)
                // which is way below maximum value of f64, so the cast is fine
                let total_stake = mix.total_bond().unwrap_or_default() as f64;
                (mix.mix_node.identity_key, total_stake)
            }) // if for some reason node is invalid, treat it as 0 stake/weight
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)
            .unwrap()
            .map(|(identity, _weight)| identity.clone())
            .collect()
    }

    async fn update_rewarded_set(&self) {
        // we know the entries are not stale, as a matter of fact they were JUST updated, since we got notified
        let all_nodes = self.validator_cache.mixnodes().await.into_inner();
        let rewarding_params = self
            .validator_cache
            .epoch_reward_params()
            .await
            .into_inner();

        let rewarded_set_size = rewarding_params.rewarded_set_size;
        let active_set_size = rewarding_params.active_set_size;

        // note that top k nodes are in the active set
        let new_rewarded_set = self.determine_rewarded_set(all_nodes, rewarded_set_size);
        if let Err(err) = self
            .nymd_client
            .write_rewarded_set(new_rewarded_set, active_set_size)
            .await
        {
            log::error!("failed to update the rewarded set - {}", err)
            // note that if the transaction failed to get executed because, I don't know, there was a networking hiccup
            // the cache will notify the updater on its next round
        }
    }

    pub(crate) async fn run(&self) {
        self.validator_cache.wait_for_initial_values().await;

        loop {
            // wait until the cache refresher determined its time to update the rewarded/active sets
            self.update_rewarded_set_notify.notified().await;
            self.update_rewarded_set().await;
        }
    }
}
