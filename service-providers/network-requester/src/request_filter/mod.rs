// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{self, Config};
use crate::error::NetworkRequesterError;
use crate::request_filter::allowed_hosts::standard_list::StandardListUpdater;
use crate::request_filter::allowed_hosts::stored_allowed_hosts::{
    start_allowed_list_reloader, StoredAllowedHosts,
};
use crate::request_filter::allowed_hosts::{OutboundRequestFilter, StandardList};
use crate::request_filter::exit_policy::ExitPolicyRequestFilter;
use log::{info, warn};
use nym_exit_policy::ExitPolicy;
use nym_socks5_requests::RemoteAddress;
use nym_task::TaskHandle;
use std::sync::Arc;

/// Old request filtering based on the allowed.list files.
pub mod allowed_hosts;
pub mod exit_policy;

enum RequestFilterInner {
    AllowList {
        open_proxy: bool,
        filter: OutboundRequestFilter,
    },
    ExitPolicy {
        policy_filter: ExitPolicyRequestFilter,
    },
}

#[derive(Clone)]
pub struct RequestFilter {
    inner: Arc<RequestFilterInner>,
}

impl RequestFilter {
    pub(crate) async fn new(config: &Config) -> Result<Self, NetworkRequesterError> {
        if config.network_requester.use_deprecated_allow_list {
            info!("setting up allow-list based 'OutboundRequestFilter'...");
            Ok(Self::new_allow_list_request_filter(config).await)
        } else {
            info!("setting up ExitPolicy based request filter...");
            Self::new_exit_policy_filter(config).await
        }
    }

    pub fn current_exit_policy_filter(&self) -> Option<&ExitPolicyRequestFilter> {
        match &*self.inner {
            RequestFilterInner::AllowList { .. } => None,
            RequestFilterInner::ExitPolicy { policy_filter } => Some(policy_filter),
        }
    }

    pub(crate) async fn start_update_tasks(
        &self,
        config: &config::Debug,
        task_handle: &TaskHandle,
    ) {
        match &*self.inner {
            RequestFilterInner::AllowList { open_proxy, filter } => {
                // if we're running in open proxy, we don't have to spawn any refreshers,
                // after all, we're going to be accepting all requests regardless
                // of the local allow list or the standard list
                if *open_proxy {
                    return;
                }

                // start the standard list updater
                StandardListUpdater::new(
                    config.standard_list_update_interval,
                    filter.standard_list(),
                    task_handle.get_handle().named("StandardListUpdater"),
                )
                .start();

                // start the allowed.list watcher and updater
                start_allowed_list_reloader(
                    filter.allowed_hosts(),
                    task_handle
                        .get_handle()
                        .named("stored_allowed_hosts_reloader"),
                )
                .await;
            }
            RequestFilterInner::ExitPolicy { .. } => {
                // nothing to do for the exit policy (yet; we might add a refresher at some point)
            }
        }
    }

    async fn new_allow_list_request_filter(config: &Config) -> Self {
        let standard_list = StandardList::new();
        let allowed_hosts = StoredAllowedHosts::new(&config.storage_paths.allowed_list_location);
        let unknown_hosts =
            allowed_hosts::HostsStore::new(&config.storage_paths.unknown_list_location);

        // TODO: technically if we're running open proxy, we don't have to be loading anything here
        RequestFilter {
            inner: Arc::new(RequestFilterInner::AllowList {
                open_proxy: config.network_requester.open_proxy,
                filter: OutboundRequestFilter::new(allowed_hosts, standard_list, unknown_hosts)
                    .await,
            }),
        }
    }

    async fn new_exit_policy_filter(config: &Config) -> Result<Self, NetworkRequesterError> {
        let policy_filter = if config.network_requester.open_proxy {
            ExitPolicyRequestFilter::new(ExitPolicy::new_open())
        } else {
            let upstream_url = config
                .network_requester
                .upstream_exit_policy_url
                .as_ref()
                .ok_or(NetworkRequesterError::NoUpstreamExitPolicy)?;
            ExitPolicyRequestFilter::new_upstream(upstream_url.clone()).await?
        };
        Ok(RequestFilter {
            inner: Arc::new(RequestFilterInner::ExitPolicy { policy_filter }),
        })
    }

    pub(crate) async fn check_address(&self, address: &RemoteAddress) -> bool {
        match &*self.inner {
            RequestFilterInner::AllowList { open_proxy, filter } => {
                if *open_proxy {
                    return true;
                }
                filter.check(address).await
            }
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
