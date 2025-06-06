// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{
    GatewayBondAnnotated, MalformedNodeBond, MixNodeBondAnnotated, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, SemiSkimmedNode, SkimmedNode};
use nym_mixnet_contract_common::reward_params::Performance;
use time::OffsetDateTime;

pub(crate) trait LegacyAnnotation {
    fn performance(&self) -> Performance;

    fn identity(&self) -> &str;

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond>;

    fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond>;
}

impl LegacyAnnotation for MixNodeBondAnnotated {
    fn performance(&self) -> Performance {
        self.node_performance.last_24h
    }

    fn identity(&self) -> &str {
        self.identity_key()
    }

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        self.try_to_skimmed_node(role)
    }

    fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond> {
        self.try_to_semi_skimmed_node(role)
    }
}

impl LegacyAnnotation for GatewayBondAnnotated {
    fn performance(&self) -> Performance {
        self.node_performance.last_24h
    }

    fn identity(&self) -> &str {
        self.identity()
    }

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        self.try_to_skimmed_node(role)
    }

    fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond> {
        self.try_to_semi_skimmed_node(role)
    }
}

pub(crate) fn refreshed_at(
    iter: impl IntoIterator<Item = OffsetDateTime>,
) -> OffsetDateTimeJsonSchemaWrapper {
    iter.into_iter()
        .min()
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .into()
}
