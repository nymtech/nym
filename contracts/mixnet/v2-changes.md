# Mixnet contract changes

This file shall describe (hopefully) all relevant changes made to the contract as the result of changing reward calculation.

## Overview

There are two main changes performed to the mixnet contract that have a cascading effect on the rest of the system. They are as follows:

1. The delegator rewarding is modified so that in order to determine the correct reward, we no longer have to iterate through data from all the epochs to correctly compound the reward. Instead, we assume there's a theoretical "unit" delegation on each mixnode that we keep track of and scale all of the actual delegation values accordingly. It's very similar to the idea presented in the [Cosmos' F1 paper](https://drops.dagstuhl.de/opus/volltexte/2020/11974/pdf/OASIcs-Tokenomics-2019-10.pdf). I've explained the entire algorithm in more details on [our gitlab](https://gitlab.nymte.ch/jstuczyn/reward-testing/-/blob/main/README.md).

2. Mixnodes are no longer stored and indexed by their identity keys. Instead, they get assigned a unique `NodeId` (just an increasing `u64` id). This is to resolve my favourite ~~bug~~ feature (I will explain this in slightly more details in the next sections) that causes rebonded mixnode to retain its delegations. With this change the following would happen:
   - new mixnode bonds, gets assigned id `X`
   - delegations are being made towards this mixnode
   - mixnode decides to unbond
   - the same mixnode rebonds, with the same identity key, same owner, same sphinx key, etc. but this time it gets assigned new id `Y`
   - as a result the delegations are still pointing to `X` and thus are no longer accumulating any rewards

While not as major as the above changes, the other notable changes include:
- introduction of `PendingEpochEvent` and `PendingIntervalEvent`. It means that whenever a relevant request is received, it's only going to get executed once the current epoch (or interval) finishes. This might include, for example, mixnode unbonding or changing mixnode cost parameters.
-

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

We have to explicitly state, as before, address of the rewarding validator which is authorized to update rewarded sets and distribute rewards to mixnodes, but also vesting contract address (since we need to know if we should call `Track...` methods), interval/epoch related parameters (so that we wouldn't accidentally try to set 10min epochs via migration :eyes:) and initial rewarding parameters that include things such as the size of the initial reward pool or the per interval emission.

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

Gateways remain mostly unchanged and unaffected by the fallout of other changes. We still keep the gateways indexed by their identity keys.

The only relevant change is that `OwnsGateway` query has been renamed to `GetOwnedGateway` to keep the naming consistent.

## Delegations

### Overview

### Types/Models

### Transactions

### Queries

### Storage



## Interval

### Overview

### Types/Models

### Transactions

### Queries

### Storage

## Contract settings

### Overview

### Types/Models

### Transactions

### Queries

### Storage



## Rewards

### Overview

### Types/Models

#### Added

#### Removed

`DelegatorRewardParams`
`StoredNodeRewardResult`
`NodeRewardResult`

#### Modified

### Transactions

### Queries

### Storage
