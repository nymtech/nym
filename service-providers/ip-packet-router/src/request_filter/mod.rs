// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::IpPacketRouterError;
use crate::request_filter::exit_policy::ExitPolicyRequestFilter;
use log::{info, warn};
use std::{net::SocketAddr, sync::Arc};

pub mod exit_policy;

enum RequestFilterInner {
    ExitPolicy {
        policy_filter: ExitPolicyRequestFilter,
    },
}

#[derive(Clone)]
pub struct RequestFilter {
    inner: Arc<RequestFilterInner>,
}

impl RequestFilter {
    pub(crate) async fn new(config: &Config) -> Result<Self, IpPacketRouterError> {
        info!("setting up ExitPolicy based request filter...");
        Self::new_exit_policy_filter(config).await
    }

    #[allow(unused)]
    pub fn current_exit_policy_filter(&self) -> Option<&ExitPolicyRequestFilter> {
        match &*self.inner {
            RequestFilterInner::ExitPolicy { policy_filter } => Some(policy_filter),
        }
    }

    pub(crate) async fn start_update_tasks(&self) {
        match &*self.inner {
            RequestFilterInner::ExitPolicy { .. } => {
                // nothing to do for the exit policy (yet; we might add a refresher at some point)
            }
        }
    }

    async fn new_exit_policy_filter(config: &Config) -> Result<Self, IpPacketRouterError> {
        let upstream_url = config
            .ip_packet_router
            .upstream_exit_policy_url
            .as_ref()
            .ok_or(IpPacketRouterError::NoUpstreamExitPolicy)?;
        let policy_filter = ExitPolicyRequestFilter::new_upstream(upstream_url.clone()).await?;
        Ok(RequestFilter {
            inner: Arc::new(RequestFilterInner::ExitPolicy { policy_filter }),
        })
    }

    pub(crate) async fn check_address(&self, address: &SocketAddr) -> bool {
        match &*self.inner {
            RequestFilterInner::ExitPolicy { policy_filter } => {
                match policy_filter.check(address).await {
                    Err(err) => {
                        warn!("failed to validate '{address}' against the exit policy: {err}");
                        false
                    }
                    Ok(res) => res,
                }
            }
        }
    }
}
