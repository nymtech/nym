// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{
    GatewayBondAnnotated, MalformedNodeBond, MixNodeBondAnnotated, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, SkimmedNode};
use nym_bin_common::version_checker;
use nym_mixnet_contract_common::reward_params::Performance;
use time::OffsetDateTime;

pub(crate) trait LegacyAnnotation {
    fn version(&self) -> &str;

    fn performance(&self) -> Performance;

    fn identity(&self) -> &str;

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond>;
}

impl LegacyAnnotation for MixNodeBondAnnotated {
    fn version(&self) -> &str {
        self.version()
    }

    fn performance(&self) -> Performance {
        self.node_performance.last_24h
    }

    fn identity(&self) -> &str {
        self.identity_key()
    }

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        self.try_to_skimmed_node(role)
    }
}

impl LegacyAnnotation for GatewayBondAnnotated {
    fn version(&self) -> &str {
        self.version()
    }

    fn performance(&self) -> Performance {
        self.node_performance.last_24h
    }

    fn identity(&self) -> &str {
        self.identity()
    }

    fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        self.try_to_skimmed_node(role)
    }
}

pub(crate) fn refreshed_at(
    iter: impl IntoIterator<Item = OffsetDateTime>,
) -> OffsetDateTimeJsonSchemaWrapper {
    iter.into_iter().min().unwrap().into()
}

pub(crate) fn semver(requirement: &Option<String>, declared: &str) -> bool {
    if let Some(semver_compat) = requirement.as_ref() {
        if !version_checker::is_minor_version_compatible(declared, semver_compat) {
            return false;
        }
    }
    true
}
