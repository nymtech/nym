// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod families_purge {
    use cosmwasm_std::{DepsMut, Order, StdResult};
    use cw_storage_plus::{Index, IndexList, IndexedMap, Map, UniqueIndex};
    use mixnet_contract_common::error::MixnetContractError;
    use nym_contracts_common::IdentityKey;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub const FAMILIES_INDEX_NAMESPACE: &str = "faml2";
    pub const FAMILIES_MAP_NAMESPACE: &str = "fam2";
    pub const MEMBERS_MAP_NAMESPACE: &str = "memb2";

    type FamilyHeadKey = IdentityKey;

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Family {
        /// Owner of this family.
        head: FamilyHead,

        /// Optional proxy (i.e. vesting contract address) used when creating the family.
        proxy: Option<String>,

        /// Human readable label for this family.
        label: String,
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct FamilyHead(IdentityKey);

    impl Serialize for FamilyHead {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.0.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for FamilyHead {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let inner = IdentityKey::deserialize(deserializer)?;
            Ok(FamilyHead(inner))
        }
    }

    pub struct FamilyIndex<'a> {
        pub label: UniqueIndex<'a, FamilyHeadKey, Family>,
    }

    impl IndexList<Family> for FamilyIndex<'_> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Family>> + '_> {
            let v: Vec<&dyn Index<Family>> = vec![&self.label];
            Box::new(v.into_iter())
        }
    }

    pub fn families<'a>() -> IndexedMap<'a, FamilyHeadKey, Family, FamilyIndex<'a>> {
        let indexes = FamilyIndex {
            label: UniqueIndex::new(|d| d.label.to_string(), FAMILIES_INDEX_NAMESPACE),
        };
        IndexedMap::new(FAMILIES_MAP_NAMESPACE, indexes)
    }

    pub const MEMBERS: Map<IdentityKey, FamilyHead> = Map::new(MEMBERS_MAP_NAMESPACE);

    pub(crate) fn families_purge(deps: DepsMut) -> Result<(), MixnetContractError> {
        // we don't care about values, we are only concerned with keys
        let family_keys = families()
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        for family in family_keys {
            families().remove(deps.storage, family)?;
        }

        let member_keys = MEMBERS
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        for member in member_keys {
            MEMBERS.remove(deps.storage, member);
        }

        Ok(())
    }
}

mod nym_nodes_usage {
    use crate::constants::{CONTRACT_STATE_KEY, REWARDING_PARAMS_KEY};
    use crate::interval::storage::current_interval;
    use crate::mixnet_contract_settings::storage::CONTRACT_STATE;
    use crate::nodes::storage::helpers::RoleStorageBucket;
    use crate::nodes::storage::rewarded_set::{ACTIVE_ROLES_BUCKET, ROLES, ROLES_METADATA};
    use crate::rewards::storage::RewardingStorage;
    use crate::support::helpers::ensure_epoch_in_progress_state;
    use cosmwasm_std::{Addr, Coin, DepsMut, Order, StdResult, Storage};
    use cw_storage_plus::{Item, Map};
    use mixnet_contract_common::error::MixnetContractError;
    use mixnet_contract_common::nym_node::{RewardedSetMetadata, Role};
    use mixnet_contract_common::reward_params::RewardedSetParams;
    use mixnet_contract_common::{
        ContractState, ContractStateParams, IntervalRewardParams, MigrateMsg, NodeId,
        OperatingCostRange, PendingIntervalEvent, PendingIntervalEventKind, ProfitMarginRange,
        RewardingParams,
    };
    use serde::{Deserialize, Serialize};

    fn migrate_contract_state(storage: &mut dyn Storage) -> Result<(), MixnetContractError> {
        #[derive(Serialize, Deserialize)]
        struct OldContractState {
            owner: Option<Addr>,
            rewarding_validator_address: Addr,
            vesting_contract_address: Addr,
            rewarding_denom: String,
            params: OldContractStateParams,
        }

        #[derive(Serialize, Deserialize)]
        struct OldContractStateParams {
            minimum_mixnode_delegation: Option<Coin>,
            minimum_mixnode_pledge: Coin,
            minimum_gateway_pledge: Coin,
            #[serde(default)]
            profit_margin: ProfitMarginRange,
            #[serde(default)]
            interval_operating_cost: OperatingCostRange,
        }

        let old_state_entry = Item::new(CONTRACT_STATE_KEY);
        let old_state: OldContractState = old_state_entry.load(storage)?;

        #[allow(deprecated)]
        CONTRACT_STATE.save(
            storage,
            &ContractState {
                owner: old_state.owner,
                rewarding_validator_address: old_state.rewarding_validator_address,
                vesting_contract_address: old_state.vesting_contract_address,
                rewarding_denom: old_state.rewarding_denom,
                params: ContractStateParams {
                    minimum_delegation: old_state.params.minimum_mixnode_delegation,
                    // just use the same value for nym-node pledge as we have for mixnodes
                    minimum_pledge: old_state.params.minimum_mixnode_pledge,
                    profit_margin: old_state.params.profit_margin,
                    interval_operating_cost: old_state.params.interval_operating_cost,
                },
            },
        )?;

        Ok(())
    }

    fn migrate_pending_interval_changes(
        storage: &mut dyn Storage,
    ) -> Result<(), MixnetContractError> {
        // at the time of writing this migration there were just 15 pending interval events,
        // so if we stay within this order of magnitude, it's quite safe to just grab all of them
        let events = crate::interval::storage::PENDING_INTERVAL_EVENTS
            .range(storage, None, None, Order::Ascending)
            .map(|res| res.map(|row| row.into()))
            .collect::<StdResult<Vec<PendingIntervalEvent>>>()?;

        for event in events {
            if let PendingIntervalEventKind::ChangeMixCostParams { mix_id, .. } = event.event.kind {
                let mut pending = crate::mixnodes::storage::PENDING_MIXNODE_CHANGES
                    .may_load(storage, mix_id)?
                    .unwrap_or_default();
                pending.cost_params_change = Some(event.id);
                crate::mixnodes::storage::PENDING_MIXNODE_CHANGES
                    .save(storage, mix_id, &pending)?;
            }
        }

        Ok(())
    }

    fn preassign_gateway_ids(
        storage: &mut dyn Storage,
    ) -> Result<(Option<NodeId>, Option<NodeId>), MixnetContractError> {
        // that one is a big if. we have ~100 gateways so we **might** be able to fit it within migration.
        // if not, then we'll have to do it in batches/change our approach

        let gateways = crate::gateways::storage::gateways()
            .range(storage, None, None, Order::Ascending)
            .map(|res| res.map(|row| row.1))
            .collect::<StdResult<Vec<_>>>()?;

        let mut start = None;
        let mut end = None;
        for gateway in gateways {
            let id = crate::nodes::storage::next_nymnode_id_counter(storage)?;
            if start.is_none() {
                start = Some(id)
            }
            end = Some(id);

            crate::gateways::storage::PREASSIGNED_LEGACY_IDS.save(
                storage,
                gateway.gateway.identity_key,
                &id,
            )?;
        }

        Ok((start, end))
    }

    fn cleanup_legacy_storage(
        storage: &mut dyn Storage,
    ) -> Result<Vec<NodeId>, MixnetContractError> {
        #[derive(Copy, Clone, Default, Serialize, Deserialize)]
        pub struct LayerDistribution {
            pub layer1: u64,
            pub layer2: u64,
            pub layer3: u64,
        }
        pub const LAYERS: Item<'_, LayerDistribution> = Item::new("layers");

        #[derive(Copy, Clone, Serialize, Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        pub enum RewardedSetNodeStatus {
            /// Node that is currently active, i.e. is expected to be used by clients for mixing packets.
            #[serde(alias = "Active")]
            Active,

            /// Node that is currently in standby, i.e. it's present in the rewarded set but is not active.
            #[serde(alias = "Standby")]
            Standby,
        }
        pub(crate) const REWARDED_SET: Map<NodeId, RewardedSetNodeStatus> = Map::new("rs");

        // remove explicit layer assignment -> got replaced with role assignment
        LAYERS.remove(storage);

        // remove every node from the legacy rewarded set
        let rewarded_ids = REWARDED_SET
            .keys(storage, None, None, Order::Ascending)
            .collect::<Result<Vec<_>, _>>()?;

        for &node_id in &rewarded_ids {
            REWARDED_SET.remove(storage, node_id)
        }

        Ok(rewarded_ids)
    }

    fn migrate_rewarded_set_params(storage: &mut dyn Storage) -> Result<(), MixnetContractError> {
        #[derive(Copy, Clone, Serialize, Deserialize)]
        pub struct LegacyRewardingParams {
            pub interval: IntervalRewardParams,
            pub rewarded_set_size: u32,
            pub active_set_size: u32,
        }
        pub(crate) const REWARDING_PARAMS: Item<'_, LegacyRewardingParams> =
            Item::new(REWARDING_PARAMS_KEY);

        let legacy = REWARDING_PARAMS.load(storage)?;

        // our mainnet assumption. we could work around it,
        // but what's the point of the extra logic if we might not need it?
        if legacy.rewarded_set_size != 240 || legacy.active_set_size != 240 {
            return Err(MixnetContractError::FailedMigration {
                comment: "the current active or rewarded set size is not 240 (the expected value for mainnet)".to_string(),
            });
        }

        let updated = RewardingParams {
            interval: legacy.interval,
            rewarded_set: RewardedSetParams {
                entry_gateways: 50,
                exit_gateways: 70,
                mixnodes: 120,
                standby: 0,
            },
        };

        RewardingStorage::load()
            .global_rewarding_params
            .save(storage, &updated)?;

        Ok(())
    }

    fn assign_temporary_rewarded_set(
        storage: &mut dyn Storage,
        (min_available_gateway, max_available_gateway): (Option<NodeId>, Option<NodeId>),
        current_rewarded_set_mixnodes: Vec<NodeId>,
    ) -> Result<(), MixnetContractError> {
        let epoch_id = current_interval(storage)?.current_epoch_absolute_id();

        // in the previous step we explicitly set rewarded set to 120 mixnodes and 50 entry gateways
        // note: we can't assign exit gateways because the contract itself doesn't know which might support it

        let active_bucket = RoleStorageBucket::default();
        let inactive_bucket = active_bucket.other();
        ACTIVE_ROLES_BUCKET.save(storage, &active_bucket)?;

        // ACTIVE BUCKET:
        let mut active_metadata = RewardedSetMetadata::new(epoch_id);

        let mut current_rewarded_set_mixnodes = current_rewarded_set_mixnodes;
        // ensure it's sorted. it should have already been, but better safe than sorry..
        current_rewarded_set_mixnodes.sort();

        let mut layer1 = Vec::new();
        let mut layer2 = Vec::new();
        let mut layer3 = Vec::new();
        let mut entry = Vec::new();

        for (i, mix_id) in current_rewarded_set_mixnodes
            .into_iter()
            .take(120)
            .enumerate()
        {
            if i % 3 == 0 {
                layer1.push(mix_id);
            } else if i % 3 == 1 {
                layer2.push(mix_id);
            } else if i % 3 == 2 {
                layer3.push(mix_id);
            }
        }

        if let (Some(min_id), Some(max_id)) = (min_available_gateway, max_available_gateway) {
            // we can assign the gateway nodes
            entry = (min_id..=max_id).take(50).collect();
        }

        // ACTIVE BUCKET:
        active_metadata.fully_assigned = true;

        // layer1
        ROLES.save(storage, (active_bucket as u8, Role::Layer1), &layer1)?;
        active_metadata.layer1_metadata.num_nodes = layer1.len() as u32;
        active_metadata.layer1_metadata.highest_id = layer1.last().copied().unwrap_or_default();

        // layer2
        ROLES.save(storage, (active_bucket as u8, Role::Layer2), &layer2)?;
        active_metadata.layer2_metadata.num_nodes = layer2.len() as u32;
        active_metadata.layer2_metadata.highest_id = layer2.last().copied().unwrap_or_default();

        // layer3
        ROLES.save(storage, (active_bucket as u8, Role::Layer3), &layer3)?;
        active_metadata.layer3_metadata.num_nodes = layer3.len() as u32;
        active_metadata.layer3_metadata.highest_id = layer3.last().copied().unwrap_or_default();

        // entry
        ROLES.save(storage, (active_bucket as u8, Role::EntryGateway), &entry)?;
        active_metadata.entry_gateway_metadata.num_nodes = entry.len() as u32;
        active_metadata.entry_gateway_metadata.highest_id =
            entry.last().copied().unwrap_or_default();

        // nothing for exit or standby
        ROLES.save(storage, (active_bucket as u8, Role::ExitGateway), &vec![])?;
        ROLES.save(storage, (active_bucket as u8, Role::Standby), &vec![])?;
        ROLES_METADATA.save(storage, active_bucket as u8, &active_metadata)?;

        // SECONDARY BUCKET
        let roles = vec![
            Role::Layer1,
            Role::Layer2,
            Role::Layer3,
            Role::EntryGateway,
            Role::ExitGateway,
            Role::Standby,
        ];
        for role in roles {
            ROLES.save(storage, (inactive_bucket as u8, role), &vec![])?
        }

        ROLES_METADATA.save(storage, inactive_bucket as u8, &Default::default())?;

        Ok(())
    }

    pub(crate) fn migrate_to_nym_nodes_usage(
        deps: DepsMut<'_>,
        _msg: &MigrateMsg,
    ) -> Result<(), MixnetContractError> {
        // ensure we're not migrating mid-epoch progression, or we're gonna have bad time
        ensure_epoch_in_progress_state(deps.storage)?;

        // update the contract state structure (remove separate mixnode/gateway pledge amount)
        migrate_contract_state(deps.storage)?;

        // make sure to assign pending cost params changes to mixnodes so those nodes couldn't be migrated
        // to nym-nodes until the events are resolved
        migrate_pending_interval_changes(deps.storage)?;

        // pre-assign NodeId to all gateways (that will be applied during nym-node migration)
        // to simplify all other code during the intermediate period
        let gateways = preassign_gateway_ids(deps.storage)?;

        // update the simple active/rewarded set sizes to actually contain the distribution of roles
        migrate_rewarded_set_params(deps.storage)?;

        // remove all redundant storage items
        let old_rewarded_set_mixnodes = cleanup_legacy_storage(deps.storage)?;

        // assign initial rewarded set
        // and initialise all the storage structures required by nym-nodes
        // based on the nodes that are in the contract right now
        assign_temporary_rewarded_set(deps.storage, gateways, old_rewarded_set_mixnodes)?;

        Ok(())
    }
}

pub(crate) use families_purge::families_purge;
pub(crate) use nym_nodes_usage::migrate_to_nym_nodes_usage;
