// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::{EpochEventId, EpochId, Gateway, IntervalEventId, MixNode, NodeId, NodeRewarding};
use contracts_common::IdentityKey;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, StdError, StdResult};
use cw_storage_plus::{IntKey, Key, KeyDeserialize, PrimaryKey};
use std::fmt::{Display, Formatter};

#[cw_serde]
#[derive(PartialOrd, Copy, Hash, Eq)]
#[repr(u8)]
pub enum Role {
    #[serde(rename = "eg", alias = "entry", alias = "entry_gateway")]
    EntryGateway = 0,

    #[serde(rename = "l1", alias = "layer1")]
    Layer1 = 1,

    #[serde(rename = "l2", alias = "layer2")]
    Layer2 = 2,

    #[serde(rename = "l3", alias = "layer3")]
    Layer3 = 3,

    #[serde(rename = "xg", alias = "exit", alias = "exit_gateway")]
    ExitGateway = 4,

    #[serde(rename = "stb", alias = "standby")]
    Standby = 128,
}

impl TryFrom<u8> for Role {
    type Error = MixnetContractError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            n if n == Role::EntryGateway as u8 => Ok(Role::EntryGateway),
            n if n == Role::Layer1 as u8 => Ok(Role::Layer1),
            n if n == Role::Layer2 as u8 => Ok(Role::Layer2),
            n if n == Role::Layer3 as u8 => Ok(Role::Layer3),
            n if n == Role::ExitGateway as u8 => Ok(Role::ExitGateway),
            n if n == Role::Standby as u8 => Ok(Role::Standby),
            n => Err(MixnetContractError::UnknownRoleRepresentation { got: n }),
        }
    }
}

impl<'a> PrimaryKey<'a> for Role {
    type Prefix = <u8 as PrimaryKey<'a>>::Prefix;
    type SubPrefix = <u8 as PrimaryKey<'a>>::SubPrefix;
    type Suffix = <u8 as PrimaryKey<'a>>::Suffix;
    type SuperSuffix = <u8 as PrimaryKey<'a>>::SuperSuffix;

    fn key(&self) -> Vec<Key> {
        // I'm not sure why it wasn't possible to delegate the call to
        // `(*self as u8).key()` directly...
        // I guess because of the `Key::Ref(&'a [u8])` variant?
        vec![Key::Val8((*self as u8).to_cw_bytes())]
    }

    fn joined_key(&self) -> Vec<u8> {
        (*self as u8).joined_key()
    }

    fn joined_extra_key(&self, key: &[u8]) -> Vec<u8> {
        (*self as u8).joined_extra_key(key)
    }
}

impl KeyDeserialize for Role {
    type Output = Role;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        let u8_key: <u8 as KeyDeserialize>::Output = <u8 as KeyDeserialize>::from_vec(value)?;
        Role::try_from(u8_key).map_err(|err| StdError::generic_err(err.to_string()))
    }

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        let u8_key: <u8 as KeyDeserialize>::Output = <u8 as KeyDeserialize>::from_slice(value)?;
        Role::try_from(u8_key).map_err(|err| StdError::generic_err(err.to_string()))
    }
}

impl Role {
    pub fn first() -> Role {
        Role::ExitGateway
    }

    pub fn next(&self) -> Option<Self> {
        // roles have to be assigned in the following order:
        // exit -> entry -> l1 -> l2 -> l3 -> standby
        match self {
            Role::ExitGateway => Some(Role::EntryGateway),
            Role::EntryGateway => Some(Role::Layer1),
            Role::Layer1 => Some(Role::Layer2),
            Role::Layer2 => Some(Role::Layer3),
            Role::Layer3 => Some(Role::Standby),
            Role::Standby => None,
        }
    }

    pub fn is_first(&self) -> bool {
        self == &Role::first()
    }

    pub fn is_standby(&self) -> bool {
        matches!(self, Role::Standby)
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Layer1 => write!(f, "mix layer 1"),
            Role::Layer2 => write!(f, "mix layer 2"),
            Role::Layer3 => write!(f, "mix layer 3"),
            Role::EntryGateway => write!(f, "entry gateway"),
            Role::ExitGateway => write!(f, "exit gateway"),
            Role::Standby => write!(f, "standby"),
        }
    }
}

/// Metadata associated with the rewarded set.
#[cw_serde]
#[derive(Default, Copy)]
pub struct RewardedSetMetadata {
    /// Epoch that this data corresponds to.
    pub epoch_id: EpochId,

    /// Indicates whether all roles got assigned to the set for this epoch.
    pub fully_assigned: bool,

    /// Metadata for the 'EntryGateway' role
    pub entry_gateway_metadata: RoleMetadata,

    /// Metadata for the 'ExitGateway' role
    pub exit_gateway_metadata: RoleMetadata,

    /// Metadata for the 'Layer1' role
    pub layer1_metadata: RoleMetadata,

    /// Metadata for the 'Layer2' role
    pub layer2_metadata: RoleMetadata,

    /// Metadata for the 'Layer3' role
    pub layer3_metadata: RoleMetadata,

    /// Metadata for the 'Standby' role
    pub standby_metadata: RoleMetadata,
}

impl RewardedSetMetadata {
    pub fn new(epoch_id: EpochId) -> Self {
        RewardedSetMetadata {
            epoch_id,
            fully_assigned: false,
            entry_gateway_metadata: Default::default(),
            exit_gateway_metadata: Default::default(),
            layer1_metadata: Default::default(),
            layer2_metadata: Default::default(),
            layer3_metadata: Default::default(),
            standby_metadata: Default::default(),
        }
    }

    pub fn set_role_count(&mut self, role: Role, num_nodes: u32) {
        match role {
            Role::EntryGateway => self.entry_gateway_metadata.num_nodes = num_nodes,
            Role::Layer1 => self.layer1_metadata.num_nodes = num_nodes,
            Role::Layer2 => self.layer2_metadata.num_nodes = num_nodes,
            Role::Layer3 => self.layer3_metadata.num_nodes = num_nodes,
            Role::ExitGateway => self.exit_gateway_metadata.num_nodes = num_nodes,
            Role::Standby => self.standby_metadata.num_nodes = num_nodes,
        }
    }

    pub fn set_highest_id(&mut self, highest_id: NodeId, role: Role) {
        match role {
            Role::EntryGateway => self.entry_gateway_metadata.highest_id = highest_id,
            Role::Layer1 => self.layer1_metadata.highest_id = highest_id,
            Role::Layer2 => self.layer2_metadata.highest_id = highest_id,
            Role::Layer3 => self.layer3_metadata.highest_id = highest_id,
            Role::ExitGateway => self.exit_gateway_metadata.highest_id = highest_id,
            Role::Standby => self.standby_metadata.highest_id = highest_id,
        }
    }

    // important note: this currently does **NOT** include gateway role as they're not being rewarded
    // and the metadata is primarily used for data lookup during epoch transition
    pub fn highest_rewarded_id(&self) -> NodeId {
        let mut highest = 0;
        if self.layer1_metadata.highest_id > highest {
            highest = self.layer1_metadata.highest_id;
        }
        if self.layer2_metadata.highest_id > highest {
            highest = self.layer2_metadata.highest_id;
        }
        if self.layer3_metadata.highest_id > highest {
            highest = self.layer3_metadata.highest_id;
        }
        if self.standby_metadata.highest_id > highest {
            highest = self.standby_metadata.highest_id;
        }

        highest
    }
}

/// Metadata associated with particular node role.
#[cw_serde]
#[derive(Default, Copy)]
pub struct RoleMetadata {
    /// Highest, also latest, node-id of a node assigned this role.
    pub highest_id: NodeId,

    /// Number of nodes assigned this particular role.
    pub num_nodes: u32,
}

/// Full details associated with given node.
#[cw_serde]
pub struct NymNodeDetails {
    /// Basic bond information of this node, such as owner address, original pledge, etc.
    pub bond_information: NymNodeBond,

    /// Details used for computation of rewarding related data.
    pub rewarding_details: NodeRewarding,

    /// Adjustments to the node that are scheduled to happen during future epoch/interval transitions.
    pub pending_changes: PendingNodeChanges,
}

impl NymNodeDetails {
    pub fn new(
        bond_information: NymNodeBond,
        rewarding_details: NodeRewarding,
        pending_changes: PendingNodeChanges,
    ) -> Self {
        NymNodeDetails {
            bond_information,
            rewarding_details,
            pending_changes,
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.bond_information.node_id
    }

    pub fn is_unbonding(&self) -> bool {
        self.bond_information.is_unbonding
    }

    pub fn original_pledge(&self) -> &Coin {
        &self.bond_information.original_pledge
    }

    pub fn pending_operator_reward(&self) -> Coin {
        let pledge = self.original_pledge();
        self.rewarding_details.pending_operator_reward(pledge)
    }

    pub fn pending_detailed_operator_reward(&self) -> StdResult<Decimal> {
        let pledge = self.original_pledge();
        self.rewarding_details
            .pending_detailed_operator_reward(pledge)
    }

    pub fn total_stake(&self) -> Decimal {
        self.rewarding_details.node_bond()
    }

    pub fn pending_pledge_change(&self) -> Option<EpochEventId> {
        self.pending_changes.pledge_change
    }
}

///
#[cw_serde]
pub struct NymNodeBond {
    /// Unique id assigned to the bonded node.
    pub node_id: NodeId,

    /// Address of the owner of this nym-node.
    pub owner: Addr,

    /// Original amount pledged by the operator of this node.
    pub original_pledge: Coin,

    /// Block height at which this nym-node has been bonded.
    pub bonding_height: u64,

    /// Flag to indicate whether this node is in the process of unbonding,
    /// that will conclude upon the epoch finishing.
    pub is_unbonding: bool,

    /// Information provided by the operator for the purposes of bonding.
    pub node: NymNode,
}

impl NymNodeBond {
    pub fn new(
        node_id: NodeId,
        owner: Addr,
        original_pledge: Coin,
        node: impl Into<NymNode>,
        bonding_height: u64,
    ) -> NymNodeBond {
        Self {
            node_id,
            owner,
            original_pledge,
            bonding_height,
            is_unbonding: false,
            node: node.into(),
        }
    }

    pub fn identity(&self) -> &str {
        &self.node.identity_key
    }

    pub fn ensure_bonded(&self) -> Result<(), MixnetContractError> {
        if self.is_unbonding {
            return Err(MixnetContractError::NodeIsUnbonding {
                node_id: self.node_id,
            });
        }
        Ok(())
    }
}

/// Information provided by the node operator during bonding that are used to allow other entities to use the services of this node.
#[cw_serde]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/NymNode.ts")
)]
pub struct NymNode {
    /// Network address of this nym-node, for example 1.1.1.1 or foo.mixnode.com
    /// that is used to discover other capabilities of this node.
    pub host: String,

    /// Allow specifying custom port for accessing the http, and thus self-described, api
    /// of this node for the capabilities discovery.
    pub custom_http_port: Option<u16>,

    /// Base58-encoded ed25519 EdDSA public key.
    pub identity_key: IdentityKey,
    // TODO: I don't think we want to include sphinx keys here,
    // given we want to rotate them and keeping that in sync with contract will be a PITA
}

impl NymNode {
    /// Perform naive validation of the attached identity key - makes sure it's correctly encoded
    /// and has 32 bytes (as expected from ed25519). we're not, however, checking if it's a valid curve point
    pub fn naive_ensure_valid_pubkey(&self) -> Result<(), MixnetContractError> {
        let decoded = bs58::decode(&self.identity_key)
            .into_vec()
            .map_err(|_| MixnetContractError::InvalidPubKey)?;
        if decoded.len() != 32 {
            return Err(MixnetContractError::InvalidPubKey);
        }
        Ok(())
    }

    /// Makes sure the provided host's length is at most 255 characters to prevent abuse.
    pub fn ensure_host_in_range(&self) -> Result<(), MixnetContractError> {
        if self.host.len() > 255 {
            return Err(MixnetContractError::HostTooLong);
        }
        Ok(())
    }
}

impl From<MixNode> for NymNode {
    fn from(value: MixNode) -> Self {
        NymNode {
            host: value.host,
            custom_http_port: Some(value.http_api_port),
            identity_key: value.identity_key,
        }
    }
}

impl From<Gateway> for NymNode {
    fn from(value: Gateway) -> Self {
        NymNode {
            host: value.host,
            custom_http_port: None,
            identity_key: value.identity_key,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/NodeConfigUpdate.ts")
)]
#[cw_serde]
#[derive(Default)]
pub struct NodeConfigUpdate {
    pub host: Option<String>,
    // ideally this would have been `Option<Option<u16>>`, but not sure if json would have recognised it
    pub custom_http_port: Option<u16>,

    // equivalent to setting `custom_http_port` to `None`
    #[serde(default)]
    pub restore_default_http_port: bool,
}

#[cw_serde]
#[derive(Default, Copy)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/PendingNodeChanges.ts")
)]
pub struct PendingNodeChanges {
    pub pledge_change: Option<EpochEventId>,
    pub cost_params_change: Option<IntervalEventId>,
}

impl PendingNodeChanges {
    pub fn new_empty() -> PendingNodeChanges {
        PendingNodeChanges {
            pledge_change: None,
            cost_params_change: None,
        }
    }

    pub fn ensure_no_pending_pledge_changes(&self) -> Result<(), MixnetContractError> {
        if let Some(pending_event_id) = self.pledge_change {
            return Err(MixnetContractError::PendingPledgeChange { pending_event_id });
        }
        Ok(())
    }

    pub fn ensure_no_pending_params_changes(&self) -> Result<(), MixnetContractError> {
        if let Some(pending_event_id) = self.cost_params_change {
            return Err(MixnetContractError::PendingParamsChange { pending_event_id });
        }
        Ok(())
    }
}

/// Basic information of a node that used to be part of the nym network but has already unbonded.
#[cw_serde]
pub struct UnbondedNymNode {
    /// Base58-encoded ed25519 EdDSA public key.
    pub identity_key: IdentityKey,

    /// NodeId assigned to this node.
    pub node_id: NodeId,

    /// Address of the owner of this nym node.
    pub owner: Addr,

    /// Block height at which this nym node has unbonded.
    pub unbonding_height: u64,
}

/// Response containing rewarding information of a node with the provided id.
#[cw_serde]
pub struct NodeRewardingDetailsResponse {
    /// Id of the requested node.
    pub node_id: NodeId,

    /// If there exists a node with the provided id, this field contains its rewarding information.
    pub rewarding_details: Option<NodeRewarding>,
}

/// Response containing details of a node belonging to the particular owner.
#[cw_serde]
pub struct NodeOwnershipResponse {
    /// Validated address of the node owner.
    pub address: Addr,

    /// If the provided address owns a nym-node, this field contains its detailed information.
    pub details: Option<NymNodeDetails>,
}

/// Response containing details of a node with the provided id.
#[cw_serde]
pub struct NodeDetailsResponse {
    /// Id of the requested node.
    pub node_id: NodeId,

    /// If there exists a node with the provided id, this field contains its detailed information.
    pub details: Option<NymNodeDetails>,
}

/// Response containing details of a bonded node with the provided identity key.
#[cw_serde]
pub struct NodeDetailsByIdentityResponse {
    /// The identity key (base58-encoded ed25519 public key) of the node.
    pub identity_key: IdentityKey,

    /// If there exists a bonded node with the provided identity key, this field contains its detailed information.
    pub details: Option<NymNodeDetails>,
}

/// Response containing the current state of the stake saturation of a node with the provided id.
#[cw_serde]
pub struct StakeSaturationResponse {
    /// Id of the requested node.
    pub node_id: NodeId,

    /// The current stake saturation of this node that is indirectly used in reward calculation formulas.
    /// Note that it can't be larger than 1.
    pub current_saturation: Option<Decimal>,

    /// The current, absolute, stake saturation of this node.
    /// Note that as the name suggests it can be larger than 1.
    /// However, anything beyond that value has no effect on the total node reward.
    pub uncapped_saturation: Option<Decimal>,
}

/// Response containing paged list of all nym-nodes that have ever unbonded.
#[cw_serde]
pub struct PagedUnbondedNymNodesResponse {
    /// Basic information of the node such as the owner or the identity key.
    pub nodes: Vec<UnbondedNymNode>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<NodeId>,
}

/// Response containing basic information of an unbonded nym-node with the provided id.
#[cw_serde]
pub struct UnbondedNodeResponse {
    /// Id of the requested nym-node.
    pub node_id: NodeId,

    /// If there existed a nym-node with the provided id, this field contains its basic information.
    pub details: Option<UnbondedNymNode>,
}

#[cw_serde]
pub struct PagedNymNodeBondsResponse {
    /// The nym node bond information present in the contract.
    pub nodes: Vec<NymNodeBond>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<NodeId>,
}

#[cw_serde]
pub struct PagedNymNodeDetailsResponse {
    /// All nym-node details stored in the contract.
    /// Apart from the basic bond information it also contains details required for all future reward calculation
    /// as well as any pending changes requested by the operator.
    pub nodes: Vec<NymNodeDetails>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<NodeId>,
}

#[cw_serde]
pub struct EpochAssignmentResponse {
    /// Epoch that this data corresponds to.
    pub epoch_id: EpochId,

    pub nodes: Vec<NodeId>,
}

#[cw_serde]
pub struct RolesMetadataResponse {
    pub metadata: RewardedSetMetadata,
}
