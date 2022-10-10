
# Mixnet contract changes

This file shall describe (hopefully) all relevant changes made to the contract as the result of changing reward calculation.

## Overview

There are two main changes performed to the mixnet contract that have a cascading effect on the rest of the system. They are as follows:

1. The delegator rewarding is modified so that in order to determine the correct reward, we no longer have to iterate through data from all the epochs to correctly compound the reward. Instead, we assume there's a theoretical "unit" delegation on each mixnode that we keep track of and scale all the actual delegation values accordingly. It's very similar to the idea presented in the [Cosmos' F1 paper](https://drops.dagstuhl.de/opus/volltexte/2020/11974/pdf/OASIcs-Tokenomics-2019-10.pdf). I've explained the entire algorithm on an example in more details on [our gitlab](https://gitlab.nymte.ch/jstuczyn/reward-testing/-/blob/main/README.md).

2. Mixnodes are no longer stored and indexed by their identity keys. Instead, they get assigned a unique `NodeId` (just an increasing `u64` id). This is to resolve my favourite ~~bug~~ feature (I will explain this in slightly more details in the next sections) that causes rebonded mixnode to retain its delegations. With this change the following would happen:
   - new mixnode bonds, gets assigned id `X`
   - delegations are being made towards this mixnode
   - mixnode decides to unbond
   - the same mixnode rebonds, with the same identity key, same owner, same sphinx key, etc. but this time it gets assigned new id `Y`
   - as a result the delegations are still pointing to `X` and thus are no longer accumulating any rewards

While not as major as the above changes, the other notable changes include:
- introduction of `PendingEpochEvent` and `PendingIntervalEvent`. It means that whenever a relevant request is received, it's only going to get executed once the current epoch (or interval) finishes. This might include, for example, mixnode unbonding or changing mixnode cost parameters.
- rewarding parameters, such as the size of the reward pool are only updated at the end of the current **interval**. They should not get modified between epochs.
- node uptime/performance can now be a decimal value as opposed to integer between 0-100
- node operators can now set their operating costs
- profit margin can now be a decimal value as opposed to integer between 0-100
- delegation/undelegation is no longer instantaneous. They will happen at the end of corresponding epoch.

## 'Benefits' to community:
- compounding happens automatically - you no longer have to keep track of it
- node operators can now set their operating costs
- profit margin can be more granular
- operators can update their basic configuration without rebonding
- if mixnode unbonds, it loses all of its delegations (permanently)

## Instantiation

The `InstantiateMsg` contains more fields to remove dependency on our "beloved" `NETWORK_DEFAULTS` and constants. Now we have to explicitly specify relevant parameters during instantiation:
```rust
pub struct InstantiateMsg {
    pub rewarding_validator_address: String,
    pub vesting_contract_address: String,
    pub rewarding_denom: String,
    pub epochs_in_interval: u32,
    pub epoch_duration: Duration,
    pub initial_rewarding_params: InitialRewardingParams,
}
```

We have to explicitly state, as before, address of the rewarding validator which is authorized to update rewarded sets and distribute rewards to mixnodes, but also vesting contract address (since we need to know if we should call `Track...` methods. Without it we could end up attempt to call a vesting contract method on a non-contract address), interval/epoch related parameters (so that we wouldn't accidentally try to set 10min epochs via migration :eyes:) and initial rewarding parameters that include things such as the size of the initial reward pool or the per interval emission.

## Mixnodes

### Overview

The mixnode part (apart from the delegations themselves) is the one most heavily affected by the changes made.

- All the associated structs were either modified or completely replaced, for example our top level `MixNodeBond` has been superseded by `MixNodeDetails` that _mostly_ contains what the original `MixNodeBond` had, but also holds additional information regarding rewarding parameters.
- Furthermore, as mentioned before, the indexing works differently now. We use `NodeId` to identify nodes as opposed to `IdentityKey`.
- However, our unique indices are still in place, i.e. it's impossible to bond multiple mixnodes with the same identities.
- "unbonding" is no longer instantaneous, instead it is executed when the current epoch finishes.
- at the time of writing this, "bonding" is still instant, though it might change.
- operators can now set their interval operating costs alongside their profit margins
- profit margin and operating cost changes will only happen at the end of current interval
- operators can now change basic information about their mixnodes without having to unbond, this includes things such as the version or host information
- ...

### Types/Models

#### Added

##### MixNodeDetails

Top level struct containing all information about particular mixnode, i.e. public keys, host information, cost function, rewarding parameters, etc.

```rust
pub struct MixNodeDetails {
    pub bond_information: MixNodeBond,

    pub rewarding_details: MixNodeRewarding,
}
```

##### MixNodeRewarding

New struct containing information required to determine correct rewards for all delegators and the operator. Keeps track of the currently distributed rewards as well as the value of the "unit delegation".

It is stored in separate `Map`, as after mixnode finishes the unbonding process, if there are any delegators who haven't undelegated yet, we need to know the below information in order to determine their rewards correctly.
```rust
pub struct MixNodeRewarding {
    /// Information provided by the operator that influence the cost function.
    #[serde(rename = "cp")]
    pub cost_params: MixNodeCostParams,

    /// Total pledge and compounded reward earned by the node operator.
    #[serde(rename = "op")]
    pub operator: Decimal,

    /// Total delegation and compounded reward earned by all node delegators.
    #[serde(rename = "dg")]
    pub delegates: Decimal,

    /// Cumulative reward earned by the "unit delegation" since the block 0.
    #[serde(rename = "tur")]
    pub total_unit_reward: Decimal,

    /// Value of the theoretical "unit delegation" that has delegated to this mixnode at block 0.
    #[serde(rename = "ud")]
    pub unit_delegation: Decimal,

    /// Marks the epoch when this node was last rewarded so that we wouldn't accidentally attempt
    /// to reward it multiple times in the same epoch.
    #[serde(rename = "le")]
    pub last_rewarded_epoch: FullEpochId,

    // technically we don't need that field to determine reward magnitude or anything
    // but it saves on extra queries to determine if we're removing the final delegation
    // (so that we could zero the field correctly)
    #[serde(rename = "uqd")]
    pub unique_delegations: u32,
}
```

##### MixNodeCostParams

Contains all cost-function related information of a given mixnode, i.e. currently the profit margin and the operating cost (per interval). It is provided by the node operator at the time of bonding and can only be changed as an interval rolls over.

```rust
pub struct MixNodeCostParams {
    pub profit_margin_percent: Percent,

    /// Operating cost of the associated mixnode per the entire interval.
    pub interval_operating_cost: Coin,
}

```

##### UnbondedMixnode

This struct is used to keep track very basic information about nodes that have already unbonded, as we would only know their `NodeId`. It is especially useful if your delegation is pointing to an unbonded node and you wanted to know the owner of the node that decided to unbond or its identity key.

```rust
pub struct UnbondedMixnode {
    pub identity: IdentityKey,
    pub owner: Addr,
    pub unbonding_height: u64,
}
```

##### MixNodeConfigUpdate

Encapsulates information sent to the contract whenever operator wants to update basic configuration information about the bonded node.

```rust
pub struct MixNodeConfigUpdate {
    pub host: String,
    pub mix_port: u16,
    pub verloc_port: u16,
    pub http_api_port: u16,
    pub version: String,
}
```

#### Removed

##### StoredMixnodeBond

The initial idea behind `StoredMixnodeBond` was to store the total delegation separately to the operator pledge (and accumulated rewards). The same issue is now resolved with the `MixNodeBond` and `MixNodeRewarding`.

#### Modified

##### MixNodeBond

```rust
// operator information + data assigned by the contract(s)
pub struct MixNodeBond {
  //
  // ...
  //
  // `id` field has been added that contains information about the assigned `NodeId`
  // +++ pub id: NodeId,
  //
  // we no longer keep total pledge (alongside delegation) on `MixNodeBond` via the `pledge_amount`, instead we only hold the `original_pledge` which is **NEVER** modified
  // --- pub pledge_amount: Coin
  // +++ pub original_pledge: Coin,
  //
  // `block_height` has been renamed to `bonding_height` to be more explicit about the intent
  // --- pub block_height: u64
  // +++ pub bonding_height: u64,
  //
  // `is_unbonding` field has been added to indicate when the mixnode has issue the request to unbond but the epoch hasn't rolled over yet (to prevent delegations on an unbonding node)
  // +++ pub is_unbonding: bool,
  //
  // `accumulated_rewards` field is no longer required to keep track of all rewards for particular node
  // --- pub accumulated_rewards: Option<Uint128>
}
```

##### MixNode

```rust
// information provided by the operator
pub struct MixNode {
  //
  // ...
  //
  // `profit_margin_percent` has been removed and this information is now provided via `MixNodeCostParams`
  // --- pub profit_margin_percent: u8,
  // +++
}
```

##### Layer

```rust
pub enum Layer {
  // `Gateway` layer has been removed from the `Layer` enum
  //---  Gateway = 0,
  //+++
    One = 1,
    Two = 2,
    Three = 3,
}
```

### Transactions

As before, all transactions have their associated `OnBehalf` equivalent that allows them to be called from the vesting contract.

#### Added

##### UpdateMixnodeCostParams

This one allows you to update the cost parameters of your mixnode, i.e. the profit margin and the interval operating costs. Execution of this transaction will result in the creation of a `PendingIntervalEvent` that will get resolved at the end of the current interval.

#### Removed

##### CheckpointMixnodes

Due to the changes to the rewarding system, we no longer have to be checkpointing mixnodes every epoch in order to keep track of their stake/parameters at those blocks.

#### Modified

##### UpdateMixnodeConfig

Updating mixnode config allowed you to update your profit margin. This functionality has been replaced with `UpdateMixnodeCostParams` and instead `UpdateMixnodeConfig` lets you to instantaneously update basic configuration such as the host information or the node version.

##### UnbondMixnode

In general sense `UnbondMixnode` works as before, i.e. it will eventually result in the mixnode getting removed from the directory. However, it's no longer instant. Whenever the transaction is executed, it will instead push a `PendingEpochEvent` that shall get resolved at the end of the current epoch.

The only immediate effect is that `is_unbonding` field on the `MixNodeBond` is going to be set to `true` and as a result, no new delegations are going to be permitted on this node.

##### BondMixnode

The only difference made to the mixnode bonding process is that operators need to provide an additional argument, of type `MixNodeCostParams`, to specify the cost function arguments of the node.

### Queries

The most relevant thing here to note is that whenever old queries were using `mix_identity` as one of their arguments, they instead use `mix_id` of type `NodeId` (`u64`).

#### Added

##### GetUnbondedMixNodes

New query allowing to grab the details of all mixnodes (paged) that have unbonded at some point in the past.

##### GetUnbondedMixNodeInformation

Same as above, but rather than getting information for all the mixnodes, it does it for the node specified by the provided `mix_id`.

##### GetStakeSaturation

Allows to directly obtain stake saturation (i.e. of the full bond (pledge + delegations)) of given mixnode.

##### GetMixnodeRewardingDetails

Allows obtaining `MixNodeRewarding` details of a particular node that, among other things, contain total delegation towards this node or the current value of the "unit" delegation.

#### Removed

Everything that was related to node snapshotting is removed, this includes the below to queries:

##### GetMixnodeAtHeight

No longer needed due to no snapshotting.

##### GetCheckpointsForMixnode

No longer needed due to no snapshotting.

##### GetCurrentOperatorCost

In the previous version the operator cost was a constant value shared by all operators. Now it is configurable and it can be queried by getting information associated via the particular node, for example with `GetMixnodeRewardingDetails` or `GetMixnodeDetails`.

#### Modified

##### GetMixNodes => GetMixNodeBonds, GetMixNodesDetailed

Since the structure of `MixNodeBond` has changed, `GetMixNodes` is replaced by `GetMixNodeBonds` that returns all `MixNodeBond` (paged) while `GetMixNodesDetailed` returns `MixNodeDetails`, that apart from the bond also contains rewarding details.

##### GetMixnodeBond

Similarly to the above query for the bond information of given mixnode has been superseded by `GetMixnodeDetails`.

##### OwnsMixnode

`OwnsMixnode` has been renamed to `GetOwnedMixnode` to keep the naming consistent.

### Storage

#### Added

- `MIXNODE_ID_COUNTER` - as mentioned before all mixnodes are indexed by an increasing `NodeId`. This counter keep track of the current value.
- `UNBONDED_MIXNODES` - `Map`storing basic information about the mixnodes that have unbonded.

#### Removed

- memoized `TOTAL_DELEGATION` was removed. Similar functionality is achieved via `rewards_storage::MIXNODE_REWARDING`.
- `LAST_PM_UPDATE_TIME` is also removed. We no longer have to keep track of that since the profit margin updates are enforced to be happening at the end of intervals.

#### Modified

- As mentioned multiple times before, mixnodes are no longer snapshot and we index them with `NodeId` and thus instead of using `IndexedSnapshotMap<'a, IdentityKeyRef<'a>, StoredMixnodeBond, MixnodeBondIndex<'a>>` we use `IndexedMap<'a, NodeId, MixNodeBond, MixnodeBondIndex<'a>>`
- To preserve uniqueness on identity keys, `MixnodeBondIndex` now also contains `UniqueIndex` on the `identity_key`.

## Gateways

### Overview

Gateways remain mostly unchanged and unaffected by the fallout of other changes. We still keep the gateways indexed by their identity keys. This might change in the future, but for the time being, there's no reason to do anything about it.

The only relevant change is that `OwnsGateway` query has been renamed to `GetOwnedGateway` to keep the naming consistent.

## Delegation

### Overview

Apart from mixnodes, the delegations are the other part of the system most affected by the introduced changes. Structurally-wise, the differences are relatively minimal, but the logic behind them, especially concerning rewarding and reward estimation (which will be described in more details in the subsequent sections) has changed in a meaningful way.

### Types/Models

#### Added

N/A

#### Removed

N/A

#### Modified

##### Delegation

```rust
pub struct Delegation {
    //
    // ...
    //
    // `node_identity` of the associated mixnode has been replaced by its assigned `node_id` as mixnode indexing has been modified
    // --- pub node_identity: IdentityKey,
    // +++ pub node_id: NodeId,

    // Value of the "unit delegation" associated with the mixnode at the time of delegation. It's used purely for calculating rewards
  // +++ pub cumulative_reward_ratio: Decimal,

    // `block_height` has been renamed to `height`
    // --- pub block_height: u64,
    // +++ pub height: u64,
}
```

### Transactions

Apart from the changes to the mixnode indexing (i.e. `identity_key => node_id`) there's been no significant changes to the transactions involving delegations. Of course this excludes anything regarding rewards, but this part is going to have its own dedicated section below.

### Queries

The same holds true for queries. If we exclude reward estimation-related queries and changes due to the new indexing, there are hardly any changes. The only notable difference is the introduction of `GetAllDelegations` which allows one to query for all delegations in the system as opposed to being restricted to a single owner or a single mixnode. The responses are still, however, paged.

### Storage

#### Added

N/A

#### Removed

- `PENDING_DELEGATION_EVENTS` `Map` has been removed as this concept is being superseded by the `PendingEpochEvent` queue.

#### Modified

The storage key structure of delegation has been slightly adjusted compared to the previous version:
- The composite storage key no longer includes the block height as we're now able to immediately work with the potentially changed values,
- For the simplicity sake, the composite subkey created for the purpose of querying by the owner/proxy combination has been changed from being a `Vec<u8>` to instead being a base58-encoded String (of the same data). This makes it slightly easier for the clients to use it, especially in paged queries.

## Interval

### Overview

Generally we've been going back and forth with having explicit distinction between epochs and intervals and making this purely implicit. In this iteration of the contract both pieces of data are explicit. `Interval` has an associated id, etc. as well as it holds information about the current epoch, number of epochs in interval, etc.

The other notable change to how interval behaves is that we expanded the concept of particular events being executed as given epoch (or interval) rolls over. Previously this was only applicable to `PendingDelegations`.

Also, now advancing epoch happens in the same message as writing the new rewarded set, so it's impossible to perform one without the other. Speaking of updating the rewarded set, I was attempting to be smart and reduce number of storage read by not writing entries that hasn't changed (i.e. if node was `Active` and its updated status is still `Active`, don't do anything). We're about to see if this wasn't a stupid overkill...

### Types/Models

#### Added

##### PendingEpochEvent

New structure keeping track of events that shall get invoked at the end of the current **epoch** (after rewards have already been distributed).

```rust
pub enum PendingEpochEvent {
  Delegate {
    owner: Addr,
    mix_id: NodeId,
    amount: Coin,
    proxy: Option<Addr>,
  },
  Undelegate {
    owner: Addr,
    mix_id: NodeId,
    proxy: Option<Addr>,
  },
  UnbondMixnode {
    mix_id: NodeId,
  },
  UpdateActiveSetSize {
    new_size: u32,
  },
}
```

##### PendingIntervalEvent

New structure keeping track of events that shall get invoked at the end of the current **interval** (after rewards have already been distributed).

```rust
pub enum PendingIntervalEvent {
  ChangeMixCostParams {
    mix: NodeId,
    new_costs: MixNodeCostParams,
  },
  UpdateRewardingParams {
    update: IntervalRewardingParamsUpdate,
  },
  UpdateIntervalConfig {
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
  },
}
```

#### Removed

N/A

#### Modified

##### Interval

```rust
pub struct Interval {
    //
    // ...
    //
    // we're now explicitly keeping track of the expected number of epochs in the stored interval (as opposed to making it implicit via constants)
    // +++ epochs_in_interval: u32,
    //
    // we're just being explicit about what we're keeping track of
    // --- start: OffsetDateTime,
    // +++ current_epoch_start: OffsetDateTime,
    //
    // the same is true for this one
    // --- length: Duration,
    // +++ epoch_length: Duration,
    //
    // and we're also explicitly separating ids of interval and epochs. Do note that it's illegal for `current_epoch_id` to be equal or larger to `epochs_in_interval` (in that case it should roll over back to 0)
    // --- id: u32,
    // +++ id: IntervalId,
    // +++ current_epoch_id: EpochId,
}
```

### Transactions

#### Added

There's a couple of newly added transaction that allow changing rewarding-related parameters, such as the active set size or pool emission, etc. But those changes should preferably only be executed at the end of the current epoch/interval (depending on a particular change requested) so that the current rewarding interval wouldn't be affected in an unexpected way. However, the transactions include the `force_immediately` field to make the change immediate if required.

##### UpdateActiveSetSize

Allows updating the active set size. Note that the new size **must** be equal to or smaller than the current rewarded set. If `force_immediately` is not set, the change will be applied at the end of the current epoch.

##### UpdateRewardingParams

Allows updating (almost) all the other rewarding-related global parameters:

```rust
pub struct IntervalRewardingParamsUpdate {
  pub reward_pool: Option<Decimal>,
  pub staking_supply: Option<Decimal>,

  pub sybil_resistance_percent: Option<Percent>,
  pub active_set_work_factor: Option<Decimal>,
  pub interval_pool_emission: Option<Percent>,
  pub rewarded_set_size: Option<u32>,
}
```

Note that at least a single change must be specified. If `force_immediately` is not set, the change will be applied at the end of the current interval.

##### UpdateIntervalConfig

Allows adjusting configuration of the interval, i.e. number of epochs it contains as well as the duration of the epochs themselves. Similarly to the above, if `force_immediately` is not set, the change will be applied at the end of the current interval.

##### ReconcileEpochEvents

Serves a very similar purpose to the removed `ReconcileDelegations`. But rather than being limited to just delegation creation/removal, this transaction would attempt to execute all pending epoch and interval events.

Do note that if current epoch is in **NOT** in progress, nothing is going to happen. Furthermore, interval events will only get executed if apart from the current epoch being over, the interval itself is over.

Anyone willing to pay the associated gas costs is can call this transaction. It's not limited to the `owner` account.

#### Removed

##### InitEpoch

Since we're going to be creating a brand-new contract, we no longer have to separately initialise the epoch (interval). It's going to be performed implicitly during contract instantiation.

##### ReconcileDelegations

As explained before, superseded by `ReconcileEpochEvents`.

##### CheckpointMixnodes

As explained multiple times before, due to the changes to the reward calculation algorithm, we no longer have to keep track of the state of all the mixnodes at each epoch.

##### WriteRewardedSet

This functionality has been moved into `AdvanceCurrentEpoch` so that it would not be possible to roll over the epoch without explicitly updating the rewarded set.

##### GetRewardedSetUpdateDetails

Rewarded set is now always being updated whenever epoch rolls over

##### GetRewardedSetRefreshBlocks

Same as above

##### GetCurrentRewardedSetHeight

We no longer keep track of rewarded sets at given height thanks to the change to the reward calculation

#### Modified

##### AdvanceCurrentEpoch

The main idea behind this transaction remains unchanged - if the current epoch/interval is over, this call rolls it over to the next one. However, it is now also responsible for additional functionalities:
- updating the rewarded set to the newly provided value,
- emptying the `PendingEpochEvent` and `PendingIntervalEvent` queues if there's anything left in there -> Do note that a separate explicit call in a different transaction is preferred, since there might be a significant amount of events to go through,
- if the interval has rolled over all the pending reward pool changes from `RewardPoolChange` (that will be elaborated in the rewards section) are applied

### Queries

#### Added

##### GetCurrentIntervalDetails

Allows querying for the information about the current `Interval` alongside data on the current blocktime and whether the current epoch and interval are already over.

##### GetPendingEpochEvents

Paged query for obtaining all currently pending `PendingEpochEvents` that shall get cleared at the end of the current epoch.

##### GetPendingIntervalEvents

Paged query for obtaining all currently pending `PendingIntervalEvents` that shall get cleared at the end of the current interval.

#### Removed

##### GetEpochsInInterval

This was removed in favour of `GetCurrentIntervalDetails` that returns the same piece of data on top of additional content.

#### Modified

##### GetRewardedSet

Querying for the rewarded set no longer lets you specify the block height. It always grabs the current one.

### Storage

#### Added

Similarly to `MIXNODE_ID_COUNTER`, we keep an increasing id for epoch and interval events. Essentially we want to ensure that we'd execute them in the order they were created and we can't use block height as it's very possible multiple requests might be created in the same block height. Thus we introduce `EPOCH_EVENT_ID_COUNTER` and `INTERVAL_EVENT_ID_COUNTER` for that purpose.

Furthermore, we keep track of the ID of the most recently executed event (in both categories), so we'd known more easily if there have been any new ones pushed without having to explicitly query for them. For that end we use `LAST_PROCESSED_EPOCH_EVENT` and `LAST_PROCESSED_INTERVAL_EVENT`

Finally, rather obviously, we have to store the actual events and those are being help in `PENDING_EPOCH_EVENTS` and `PENDING_INTERVAL_EVENTS` `Map`s.

#### Removed

- `CURRENT_EPOCH_REWARD_PARAMS` - in a way superseded by `rewards_storage::REWARDING_PARAMS` that holds current rewarding parameters for the entire interval.
- `CURRENT_REWARDED_SET_HEIGHT` - we only hold a single rewarded set at a time
- `EPOCHS` - all epoch related information is included in the `Interval` data.

#### Modified

- `CURRENT_EPOCH` has been replaced by a differently named `CURRENT_INTERVAL`
- `REWARDED_SET` - the storage key no longer requires using the block height as there's only ever a single rewarded set at a time

## Contract settings

### Overview

Contract state/settings now explicitly contain information that previously was implicit via the constants or network defaults (such as the `denom`). Also, parameters affecting rewarding, such as the active set size, were moved to more appropriate modules.

### Types/Models

#### Added

N/A

#### Removed

N/A

#### Modified

##### ContractState

```rust
pub struct ContractState {
    //
    // ...
    //
    // we're explicitly keeping track of what we think is the vesting contract (as specified during instantiation), so that we'd known if we should call `Track...` methods on the proxy address
    // +++ pub vesting_contract_address: Addr,
    // added information about the expected coin denomination that's used for rewarding
    // +++ pub rewarding_denom: String,
}
```

##### ContractStateParams

```rust
pub struct ContractStateParams {
    //
    // ...
    //
    // we're tracking all minimum pledges explicitly as `Coin` now
    // --- pub minimum_mixnode_pledge: Uint128
    // +++ pub minimum_mixnode_pledge: Coin,
    //
    // --- pub minimum_gateway_pledge: Uint128,
    // +++ pub minimum_gateway_pledge: Coin,
    //
    // optional functionality to set minimum delegation amount if required
    // +++ pub minimum_mixnode_delegation: Option<Coin>,
    //
    // Attributes directly affecting rewarding are moved to `rewards_storage` now
    // --- pub mixnode_rewarded_set_size: u32,
    // --- pub mixnode_active_set_size: u32,
    // --- pub staking_supply: Uint128,
}
```

### Transactions

The only transaction, i.e. updating state params, is no longer a unit enum. It was changed from
```rust
pub enum ExecuteMsg {
    UpdateContractStateParams(ContractStateParams),
}
```

to
```rust
pub enum ExecuteMsg {
    UpdateContractStateParams {
        updated_parameters: ContractStateParams,
    },
}
```

### Queries

#### Added

##### GetState

Introduced new query to get the entire `ContractState` struct, so we'd known about, for example, the rewarding denom or the rewarding validator address.

#### Removed

N/A

#### Modified

##### StateParams

Was renamed to `GetStateParams` to keep naming consistent

### Storage

No relevant changes were performed to the storage structure of the contract settings.

## Rewards

### Overview

Changing the logic behind the rewards was the main motivation behind this new contract version. The main things in this section, apart from what was already mentioned before, include but is not limited to:
- reward pool (and the staking supply) being only updated at the end of the given interval. However, the accounting is still happening as the rewards are distributed, so we'd known how much the pool should be adjusted by.
- we no longer keep any historical information regarding past epochs/parameters/etc for the purposes of rewarding. Whatever exists in the storage at the time is the thing that's going to be used for the next distribution.
- queries for reward estimation now require constant(ish) amount of gas as opposed to growing linearly with the number of epochs since last claim/compounding.
-

### Types/Models

#### Added

##### MixNodeRewarding

The most important struct created for the purposes of the changes described. All the data here allows us to correctly determine rewards for all the delegators by scaling the value of `total_unit_reward` based on the ratio the delegation to `unit_delegation` and scaled by the unit delegation reward at the time of delegation of the delegate.

```rust
pub struct MixNodeRewarding {
    /// Information provided by the operator that influence the cost function.
    pub cost_params: MixNodeCostParams,

    /// Total pledge and compounded reward earned by the node operator.
    pub operator: Decimal,

    /// Total delegation and compounded reward earned by all node delegators.
    pub delegates: Decimal,

    /// Cumulative reward earned by the "unit delegation" since the block 0.
    pub total_unit_reward: Decimal,

    /// Value of the theoretical "unit delegation" that has delegated to this mixnode at block 0.
    pub unit_delegation: Decimal,

    /// Marks the epoch when this node was last rewarded so that we wouldn't accidentally attempt
    /// to reward it multiple times in the same epoch.
    pub last_rewarded_epoch: FullEpochId,

    // technically we don't need that field to determine reward magnitude or anything
    // but it saves on extra queries to determine if we're removing the final delegation
    // (so that we could zero the field correctly)
    pub unique_delegations: u32,
}
```

##### RewardPoolChange

Whenever we distribute rewards, we keep track of how much should get removed from the reward pool and moved into the staking supply when the interval finishes.

```rust
pub(crate) struct RewardPoolChange {
    /// Indicates amount that shall get moved from the reward pool to the staking supply
    /// upon the current interval finishing.
    pub removed: Decimal,

    // this will be used once coconut credentials are in use;
    /// Indicates amount that shall get added to the both reward pool and not touch the staking supply
    /// upon the current interval finishing.
    #[allow(unused)]
    pub added: Decimal,
}
```

##### RewardingParams and IntervalRewardParams

Those are used for keeping track of parameters used for rewarding of all nodes during a particular interval. Unless there's an exceptionally good reason for it, they remain constants within an interval.

```rust
pub struct RewardingParams {
  /// Parameters that should remain unchanged throughout an interval.
  pub interval: IntervalRewardParams,

  // while the active set size can change between epochs to accommodate for bandwidth demands,
  // the active set size should be unchanged between epochs and should only be adjusted between
  // intervals. However, it makes more sense to keep both of those values together as they're
  // very strongly related to each other.
  pub rewarded_set_size: u32,
  pub active_set_size: u32,
}

pub struct IntervalRewardParams {
  /// Current value of the rewarding pool.
  /// It is expected to be constant throughout the interval.
  pub reward_pool: Decimal,

  /// Current value of the staking supply.
  /// It is expected to be constant throughout the interval.
  pub staking_supply: Decimal,

  // computed values
  /// Current value of the computed reward budget per epoch, per node.
  /// It is expected to be constant throughout the interval.
  pub epoch_reward_budget: Decimal,

  /// Current value of the stake saturation point.
  /// It is expected to be constant throughout the interval.
  pub stake_saturation_point: Decimal,

  // constants(-ish)
  // default: 30%
  /// Current value of the sybil resistance percent (`alpha`).
  /// It is not really expected to be changing very often.
  /// As a matter of fact, unless there's a very specific reason, it should remain constant.
  pub sybil_resistance: Percent,

  // default: 10
  /// Current active set work factor.
  /// It is not really expected to be changing very often.
  /// As a matter of fact, unless there's a very specific reason, it should remain constant.
  pub active_set_work_factor: Decimal,

  // default: 2%
  /// Current maximum interval pool emission.
  /// Assuming all nodes in the rewarded set are fully saturated and have 100% performance,
  /// this % of the reward pool would get distributed in rewards to all operators and its delegators.
  /// It is not really expected to be changing very often.
  /// As a matter of fact, unless there's a very specific reason, it should remain constant.
  pub interval_pool_emission: Percent,
}
```

#### Removed

##### DelegatorRewardParams, StoredNodeRewardResult, EpochRewardParams, NodeRewardResult

All reward-related results and parameters got consolidated, mostly into `RewardingParams`

### Transactions

#### Added

#### Removed

##### CompoundOperatorReward, CompoundDelegatorReward, CompoundOperatorRewardOnBehalf, CompoundDelegatorRewardOnBehalf

All compounding-related transactions have been removed as they're no longer required since the compounding is happening automatically now.

#### Modified

##### RewardMixnode

As with previous changes, we're now rewarding given mixnode by its `NodeId` as opposed to `IdentityKey`. Furthermore, we no longer have to pass entire set of `NodeRewardParams`. Everything is implicit from the contract state, with the single exception of node `Performance`, which is now required. However, the strong typing ensures its always in the correct range.

##### ClaimOperator/DelegatorReward

All claim-related operations have been renamed to `Withdraw` for consistency with cosmos-sdk. It essentially zeroes your reward and moves this amount to your account address.

### Queries

#### Added

##### GetPendingMixNodeOperatorReward

Added variant of obtaining pending operator reward by the id of the bonded mixnode.

#### Removed

##### GetRewardPool, GetCirculatingSupply, GetStakingSupply, GetIntervalRewardPercent, GetSybilResistancePercent, GetActiveSetWorkFactor

All the queries regarding individual rewarding parameters have been consolidated into a single `GetRewardingParams`

#### Modified

##### QueryOperatorReward

Renamed to `GetPendingOperatorReward` for consistency' sake.

##### QueryDelegatorReward

Renamed to `GetPendingDelegatorReward` for consistency' sake.

### Storage

As with everything in this module, storage was also completely revamped. The changes here mostly follow on the changes to data structs.

#### Added

- `REWARDING_PARAMS` - all the rewarding parameters are consolidated in a single `Item`
- `PENDING_REWARD_POOL_CHANGE` - keeping track of the reward pool changes that shall get applied at the end of the interval
- `MIXNODE_REWARDING` - per mixnode, indexed by `NodeId`, parameters required to determine operator and delegates rewards

#### Removed

- `REWARD_POOL` - incorporated into `REWARDING_PARAMS`
- `REWARDING_STATUS` - it was already deprecated to begin with since we're no longer explicitly rewarding delegators,
- `DELEGATOR_REWARD_CLAIMED_HEIGHT`, `OPERATOR_REWARD_CLAIMED_HEIGHT` - due to auto-compounding, we don't have to keep track of heights of reward claiming
- `EPOCH_REWARD_PARAMS` - we no longer have to retroactively determine rewards for past epochs and thus we no longer have to keep track of rewarding params for past epochs

## Final remarks

As mentioned during multiple chats, I think the migration the rest of our codebase is going to be a huge undertaking mostly because of how many aspects of the system this change is affecting. From the top of my head, we'd need to definitely change our `nymd client` (and as a result `validator-api`, `clients`, etc.) and also the vesting contract.

With the latter case (and with the current mixnet contract), it's going to be even trickier given that the current contract is already live. We will need to adjust how the values are stored, i.e. mixnodes are now indexed by `NodeId` as opposed to `IdentityKey`. My recommendation would be to create a migration such that it would "cancel" / "return" (you name it) all existing delegations and bonds so that the users would have to make new ones under the new contract.
