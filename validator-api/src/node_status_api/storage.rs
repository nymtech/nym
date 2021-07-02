// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, MixnodeStatusReport, NodeStatusApiError,
};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub(crate) struct NodeStatusStorage {
    // to be determined if it will hold sled/sqlite/whatever ...
    inner: Arc<RwLock<Inner>>,
}

impl NodeStatusStorage {
    pub(crate) fn new() -> Self {
        NodeStatusStorage {
            inner: Arc::new(RwLock::new(Inner::new())),
        }
    }
    pub(crate) async fn get_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, NodeStatusApiError> {
        Ok(MixnodeStatusReport::example())
    }

    pub(crate) async fn get_gateway_report(
        &self,
        identity: &str,
    ) -> Result<GatewayStatusReport, NodeStatusApiError> {
        Err(NodeStatusApiError::GatewayReportNotFound(
            identity.to_string(),
        ))
    }

    pub(crate) async fn get_all_mixnode_reports(
        &self,
    ) -> Result<Vec<MixnodeStatusReport>, NodeStatusApiError> {
        todo!()
    }

    pub(crate) async fn get_all_gateway_reports(
        &self,
    ) -> Result<Vec<GatewayStatusReport>, NodeStatusApiError> {
        todo!()
    }
}

struct Inner {
    //
}

impl Inner {
    fn new() -> Self {
        Inner {}
    }
}
