use std::fs::File;
use std::path::Path;

use chrono::{DateTime, Utc};
use log::info;
use nym_explorer_api_requests::NymVestingAccount;
use nym_mixnet_contract_common::{Addr, Delegation, NodeId, PendingRewardResponse};
use serde::{Deserialize, Serialize};

use crate::client::ThreadsafeValidatorClient;
use crate::geo_ip::location::ThreadsafeGeoIp;
use nym_mixnet_contract_common::Coin as CosmWasmCoin;
use nym_validator_client::models::MixNodeBondAnnotated;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, PagedMixnetQueryClient, VestingQueryClient,
};
use nym_validator_client::nyxd::{AccountId, Coin, CosmWasmClient};

use crate::country_statistics::country_nodes_distribution::{
    CountryNodesDistribution, ThreadsafeCountryNodesDistribution,
};
use crate::gateways::location::GatewayLocationCache;
use crate::gateways::models::ThreadsafeGatewayCache;
use crate::mix_node::models::ThreadsafeMixNodeCache;
use crate::mix_nodes::location::MixnodeLocationCache;
use crate::mix_nodes::models::ThreadsafeMixNodesCache;
use crate::nym_nodes::location::NymNodeLocationCache;
use crate::nym_nodes::models::ThreadSafeNymNodesCache;
use crate::ping::models::ThreadsafePingCache;
use crate::validators::models::ThreadsafeValidatorCache;

// TODO: change to an environment variable with a default value
const STATE_FILE: &str = "explorer-api-state.json";

#[derive(Clone)]
pub struct ExplorerApiState {
    pub(crate) country_node_distribution: ThreadsafeCountryNodesDistribution,
    pub(crate) gateways: ThreadsafeGatewayCache,
    pub(crate) mixnode: ThreadsafeMixNodeCache,
    pub(crate) mixnodes: ThreadsafeMixNodesCache,
    pub(crate) nymnodes: ThreadSafeNymNodesCache,
    pub(crate) ping: ThreadsafePingCache,
    pub(crate) validators: ThreadsafeValidatorCache,
    pub(crate) geo_ip: ThreadsafeGeoIp,

    // TODO: discuss with @MS whether this is an appropriate spot for it
    pub(crate) validator_client: ThreadsafeValidatorClient,
}

impl ExplorerApiState {
    pub(crate) async fn get_mix_node(&self, mix_id: NodeId) -> Option<MixNodeBondAnnotated> {
        self.mixnodes.get_mixnode(mix_id).await
    }

    pub(crate) async fn get_delegations_by_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<Delegation>, rocket::response::status::NotFound<String>> {
        match self
            .validator_client
            .0
            .nyxd
            .get_all_single_mixnode_delegations(node_id)
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(rocket::response::status::NotFound(format!("{}", e))),
        }
    }

    pub(crate) async fn get_balance(
        &self,
        addr: &AccountId,
    ) -> Result<Vec<Coin>, rocket::response::status::NotFound<String>> {
        match self.validator_client.0.nyxd.get_all_balances(addr).await {
            Ok(res) => Ok(res),
            Err(e) => Err(rocket::response::status::NotFound(format!("{}", e))),
        }
    }

    pub(crate) async fn get_vesting_balance(
        &self,
        addr: &AccountId,
    ) -> Result<Option<NymVestingAccount>, rocket::response::status::NotFound<String>> {
        match nym_validator_client::nyxd::contract_traits::VestingQueryClient::get_account(
            &self.validator_client.0.nyxd,
            addr.as_ref(),
        )
        .await
        {
            // 1. is there a vesting account?
            Ok(_res) => {
                // 2. there is vesting account, get all the coins
                let mut locked = CosmWasmCoin::default();
                let mut vested = CosmWasmCoin::default();
                let mut vesting = CosmWasmCoin::default();
                let mut spendable = CosmWasmCoin::default();

                // 3. try to get each coin type
                if let Ok(coin) = self
                    .validator_client
                    .0
                    .nyxd
                    .locked_coins(addr.as_ref(), None)
                    .await
                {
                    locked = coin.into();
                }
                if let Ok(coin) = self
                    .validator_client
                    .0
                    .nyxd
                    .vested_coins(addr.as_ref(), None)
                    .await
                {
                    vested = coin.into();
                }
                if let Ok(coin) = self
                    .validator_client
                    .0
                    .nyxd
                    .vesting_coins(addr.as_ref(), None)
                    .await
                {
                    vesting = coin.into();
                }
                if let Ok(coin) = self
                    .validator_client
                    .0
                    .nyxd
                    .spendable_coins(addr.as_ref(), None)
                    .await
                {
                    spendable = coin.into();
                }

                // 4.combine into a response
                Ok(Some(NymVestingAccount {
                    locked,
                    vested,
                    vesting,
                    spendable,
                }))
            }
            Err(e) => Err(rocket::response::status::NotFound(format!("{}", e))),
        }
    }

    pub(crate) async fn get_delegations(
        &self,
        addr: &AccountId,
    ) -> Result<Vec<Delegation>, rocket::response::status::NotFound<String>> {
        match self
            .validator_client
            .0
            .nyxd
            .get_all_delegator_delegations(addr)
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(rocket::response::status::NotFound(format!("{}", e))),
        }
    }

    pub(crate) async fn get_delegation_rewards(
        &self,
        addr: &AccountId,
        node_id: &NodeId,
        proxy: &Option<Addr>,
    ) -> Result<PendingRewardResponse, rocket::response::status::NotFound<String>> {
        match self
            .validator_client
            .0
            .nyxd
            .get_pending_delegator_reward(addr, *node_id, proxy.clone().map(|d| d.to_string()))
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(rocket::response::status::NotFound(format!("{}", e))),
        }
    }

    pub(crate) async fn get_operator_rewards(
        &self,
        addr: &AccountId,
    ) -> Result<PendingRewardResponse, rocket::response::status::NotFound<String>> {
        match self
            .validator_client
            .0
            .nyxd
            .get_pending_operator_reward(addr)
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(rocket::response::status::NotFound(format!("{}", e))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorerApiStateOnDisk {
    pub(crate) country_node_distribution: CountryNodesDistribution,
    pub(crate) mixnode_location_cache: MixnodeLocationCache,
    pub(crate) gateway_location_cache: GatewayLocationCache,
    pub(crate) nymnode_location_cache: NymNodeLocationCache,
    pub(crate) as_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct ExplorerApiStateContext {
    pub(crate) inner: ExplorerApiState,
}

impl ExplorerApiStateContext {
    pub(crate) fn new() -> Self {
        ExplorerApiStateContext {
            inner: ExplorerApiStateContext::read_from_file(),
        }
    }

    pub(crate) fn read_from_file() -> ExplorerApiState {
        let json_file = get_state_file_path();
        let json_file_path = Path::new(&json_file);
        info!("Loading state from file {:?}...", json_file);

        if let Ok(Ok(state)) =
            File::open(json_file_path).map(serde_json::from_reader::<_, ExplorerApiStateOnDisk>)
        {
            info!("Loaded state from file {:?}: {:?}", json_file, state);
            ExplorerApiState {
                country_node_distribution:
                    ThreadsafeCountryNodesDistribution::new_from_distribution(
                        state.country_node_distribution,
                    ),
                gateways: ThreadsafeGatewayCache::new_with_location_cache(
                    state.gateway_location_cache,
                ),
                mixnode: ThreadsafeMixNodeCache::new(),
                mixnodes: ThreadsafeMixNodesCache::new_with_location_cache(
                    state.mixnode_location_cache,
                ),
                nymnodes: ThreadSafeNymNodesCache::new_with_location_cache(
                    state.nymnode_location_cache,
                ),
                ping: ThreadsafePingCache::new(),
                validators: ThreadsafeValidatorCache::new(),
                validator_client: ThreadsafeValidatorClient::new(),
                geo_ip: ThreadsafeGeoIp::new(),
            }
        } else {
            warn!(
                "Failed to load state from file {:?}, starting with empty state!",
                json_file
            );

            ExplorerApiState {
                country_node_distribution: ThreadsafeCountryNodesDistribution::new(),
                gateways: ThreadsafeGatewayCache::new(),
                mixnode: ThreadsafeMixNodeCache::new(),
                mixnodes: ThreadsafeMixNodesCache::new(),
                nymnodes: ThreadSafeNymNodesCache::new(),
                ping: ThreadsafePingCache::new(),
                validators: ThreadsafeValidatorCache::new(),
                validator_client: ThreadsafeValidatorClient::new(),
                geo_ip: ThreadsafeGeoIp::new(),
            }
        }
    }

    pub(crate) async fn write_to_file(&self) {
        let json_file = get_state_file_path().to_string();
        let json_file_path = Path::new(&json_file);
        let file = File::create(json_file_path).expect("unable to create state json file");
        let state = ExplorerApiStateOnDisk {
            country_node_distribution: self.inner.country_node_distribution.get_all().await,
            mixnode_location_cache: self.inner.mixnodes.get_locations().await,
            gateway_location_cache: self.inner.gateways.get_locations().await,
            nymnode_location_cache: self.inner.nymnodes.get_locations().await,
            as_at: Utc::now(),
        };
        serde_json::to_writer(file, &state).expect("error writing state to disk");
        info!("Saved file to '{:?}'", json_file_path.canonicalize());
    }
}

fn get_state_file_path() -> String {
    std::env::var("API_STATE_FILE").unwrap_or_else(|_| STATE_FILE.to_string())
}
