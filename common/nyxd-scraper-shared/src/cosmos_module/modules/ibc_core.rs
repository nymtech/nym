// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{CosmosModule, MessageRegistry};
use ibc_proto::ibc::core::channel::{
    self,
    v1::{
        MsgAcknowledgement, MsgChannelCloseConfirm, MsgChannelCloseInit, MsgChannelOpenAck,
        MsgChannelOpenConfirm, MsgChannelOpenInit, MsgChannelOpenTry, MsgChannelUpgradeAck,
        MsgChannelUpgradeCancel, MsgChannelUpgradeConfirm, MsgChannelUpgradeInit,
        MsgChannelUpgradeOpen, MsgChannelUpgradeTimeout, MsgChannelUpgradeTry,
        MsgPruneAcknowledgements, MsgRecvPacket, MsgTimeout, MsgTimeoutOnClose,
    },
};
use ibc_proto::ibc::core::client::{
    self,
    v1::{
        MsgCreateClient, MsgIbcSoftwareUpgrade, MsgRecoverClient, MsgSubmitMisbehaviour,
        MsgUpdateClient, MsgUpgradeClient,
    },
};
use ibc_proto::ibc::core::connection::{
    self,
    v1::{
        MsgConnectionOpenAck, MsgConnectionOpenConfirm, MsgConnectionOpenInit, MsgConnectionOpenTry,
    },
};

pub(crate) struct IbcCore;

impl CosmosModule for IbcCore {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        // channel
        registry.register::<MsgChannelOpenInit>();
        registry.register::<MsgChannelOpenTry>();
        registry.register::<MsgChannelOpenAck>();
        registry.register::<MsgChannelOpenConfirm>();
        registry.register::<MsgChannelCloseInit>();
        registry.register::<MsgChannelCloseConfirm>();
        registry.register::<MsgRecvPacket>();
        registry.register::<MsgTimeout>();
        registry.register::<MsgTimeoutOnClose>();
        registry.register::<MsgAcknowledgement>();
        registry.register::<MsgChannelUpgradeInit>();
        registry.register::<MsgChannelUpgradeTry>();
        registry.register::<MsgChannelUpgradeAck>();
        registry.register::<MsgChannelUpgradeConfirm>();
        registry.register::<MsgChannelUpgradeOpen>();
        registry.register::<MsgChannelUpgradeTimeout>();
        registry.register::<MsgChannelUpgradeCancel>();
        registry.register::<channel::v1::MsgUpdateParams>();
        registry.register::<MsgPruneAcknowledgements>();

        // client
        registry.register::<MsgCreateClient>();
        registry.register::<MsgUpdateClient>();
        registry.register::<MsgUpgradeClient>();
        registry.register::<MsgSubmitMisbehaviour>();
        registry.register::<MsgRecoverClient>();
        registry.register::<MsgIbcSoftwareUpgrade>();
        registry.register::<client::v1::MsgUpdateParams>();

        // connection
        registry.register::<MsgConnectionOpenInit>();
        registry.register::<MsgConnectionOpenTry>();
        registry.register::<MsgConnectionOpenAck>();
        registry.register::<MsgConnectionOpenConfirm>();
        registry.register::<connection::v1::MsgUpdateParams>();
    }
}
