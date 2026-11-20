// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config;
use crate::error::NymNodeError;
use crate::node::authorised_agents::AuthorisedAgent;
use crate::node::nyxd_watcher::network_monitor_agents::NetworkMonitorAgentsModule;
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::rpc::TendermintRpcClientExt;
use nyxd_scraper_shared::watcher::{NyxdWatcher, WatcherConfig};
use rand::RngExt;
use std::time::Duration;
use tokio::time::{Instant, sleep};
use tracing::{error, info, warn};
use url::Url;

/// How many times boot tries to start a watcher before the node refuses to start.
const BOOT_ATTEMPTS: usize = 3;

/// The pause between boot attempts, giving a briefly unavailable endpoint time to come back.
const BOOT_ATTEMPT_PAUSE: Duration = Duration::from_secs(10);

/// The wait before the first attempt to start a new watcher.
const FIRST_RESTART_WAIT: Duration = Duration::from_secs(30);

/// The longest wait between attempts, and how long a watcher has to run for the wait to start over.
const MAX_RESTART_WAIT: Duration = Duration::from_secs(15 * 60);

pub(crate) struct NyxWatcherSupervisorConfig {
    websocket_url: Url,

    /// Tried in order, both for loading state and by the watcher.
    rpc_urls: Vec<Url>,
}

impl From<&config::Nyx> for NyxWatcherSupervisorConfig {
    fn from(nyx: &config::Nyx) -> Self {
        NyxWatcherSupervisorConfig {
            websocket_url: nyx.nyxd_websocket_url.clone(),
            rpc_urls: nyx.nyxd_urls_with_fallbacks(),
        }
    }
}

/// Keeps a nyx watcher running: whenever one ends, it loads every module's state pinned at the
/// current height and starts a new watcher from there.
pub(crate) struct NyxWatcherSupervisor {
    websocket_url: Url,
    endpoints: Vec<NyxEndpoint>,
    network_monitors: NetworkMonitorAgentsModule,
    backoff: RestartBackoff,
    running: Option<RunningWatcher>,
    shutdown_token: ShutdownToken,
}

impl NyxWatcherSupervisor {
    pub(crate) fn new(
        config: NyxWatcherSupervisorConfig,
        nyxd_config: nyxd::Config,
        network_monitors: NetworkMonitorAgentsModule,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        let endpoints = config
            .rpc_urls
            .into_iter()
            .map(|url| NyxEndpoint::connect(url, nyxd_config.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(NyxWatcherSupervisor {
            websocket_url: config.websocket_url,
            endpoints,
            network_monitors,
            backoff: RestartBackoff::new(),
            running: None,
            shutdown_token,
        })
    }

    /// Every module's state at the current height, from the first endpoint that serves all of it.
    async fn load_pinned_state(&self) -> Result<PinnedState, NymNodeError> {
        for endpoint in &self.endpoints {
            match self.load_pinned_state_from(endpoint).await {
                Ok(state) => return Ok(state),
                Err(err) => warn!(
                    "failed to load the chain state from {}: {err}",
                    endpoint.url
                ),
            }
        }
        Err(NymNodeError::NyxStateUnavailable)
    }

    async fn load_pinned_state_from(
        &self,
        endpoint: &NyxEndpoint,
    ) -> Result<PinnedState, NyxdError> {
        let height = endpoint.client.get_height().await?;
        let network_monitor_agents = self
            .network_monitors
            .load_at(&endpoint.client, height)
            .await?;

        Ok(PinnedState {
            height: height.value() as u32,
            network_monitor_agents,
        })
    }

    /// Load and publish every module's state, then start a watcher from the block after it.
    async fn start_watcher(&self) -> Result<RunningWatcher, NymNodeError> {
        let state = self.load_pinned_state().await?;
        self.network_monitors
            .publish(state.network_monitor_agents)
            .await;

        info!("starting the nyx watcher after height {}", state.height);
        let config = WatcherConfig {
            websocket_url: self.websocket_url.clone(),
            rpc_urls: self
                .endpoints
                .iter()
                .map(|endpoint| endpoint.url.clone())
                .collect(),
            start_after_height: Some(state.height),
        };

        // nothing may be awaited after this: `run` drops this future on shutdown, and a dropped
        // watcher handle does not stop the watcher's tasks
        let watcher = NyxdWatcher::builder(config)
            .with_msg_module(self.network_monitors.clone())
            .build_and_start()
            .await?;

        Ok(RunningWatcher {
            watcher,
            started_at: Instant::now(),
        })
    }

    /// Start the first watcher. Unless `allow_failure` is set, a node whose every attempt fails
    /// refuses to start.
    pub(crate) async fn boot(&mut self, allow_failure: bool) -> Result<(), NymNodeError> {
        let mut attempt = 1;
        loop {
            let err = match self.start_watcher().await {
                Ok(running) => {
                    self.running = Some(running);
                    return Ok(());
                }
                Err(err) => err,
            };

            if allow_failure {
                error!(
                    "failed to start the nyx watcher: {err}. Every network monitor agent is rejected until a later attempt succeeds"
                );
                return Ok(());
            }
            if attempt == BOOT_ATTEMPTS {
                return Err(err);
            }

            warn!("attempt {attempt} of {BOOT_ATTEMPTS} to start the nyx watcher failed: {err}");
            attempt += 1;
            sleep(BOOT_ATTEMPT_PAUSE).await;
        }
    }

    /// Start a new watcher whenever the running one ends, until the node shuts down.
    pub(crate) async fn run(mut self) {
        loop {
            if let Some(running) = self.running.take() {
                tokio::select! {
                    biased;
                    _ = self.shutdown_token.cancelled() => {
                        running.watcher.stop().await;
                        return;
                    }
                    _ = running.watcher.ended() => {
                        warn!("the nyx watcher has ended - stopping it before starting a new one");
                        running.watcher.stop().await;
                        self.backoff.watcher_ran_for(running.started_at.elapsed());
                    }
                }
            }

            let wait = RestartBackoff::with_jitter(self.backoff.next_wait());
            info!("starting a new nyx watcher in {}s", wait.as_secs());
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => return,
                _ = sleep(wait) => {}
            }

            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => return,
                started = self.start_watcher() => match started {
                    Ok(running) => self.running = Some(running),
                    Err(err) => warn!("failed to start a new nyx watcher: {err}"),
                }
            }
        }
    }
}

/// A chain endpoint with its url, which the client cannot report itself.
struct NyxEndpoint {
    url: Url,
    client: QueryHttpRpcNyxdClient,
}

impl NyxEndpoint {
    fn connect(url: Url, nyxd_config: nyxd::Config) -> Result<Self, NyxdError> {
        let client = QueryHttpRpcNyxdClient::connect(nyxd_config, url.as_str())?;
        Ok(NyxEndpoint { url, client })
    }
}

/// Every module's state, loaded at one height from one endpoint.
struct PinnedState {
    height: u32,
    network_monitor_agents: Vec<AuthorisedAgent>,
}

struct RunningWatcher {
    watcher: NyxdWatcher,
    started_at: Instant,
}

/// The wait before each attempt to start a new watcher.
struct RestartBackoff {
    next: Duration,
}

impl RestartBackoff {
    fn new() -> Self {
        RestartBackoff {
            next: FIRST_RESTART_WAIT,
        }
    }

    /// A watcher that ran for at least `MAX_RESTART_WAIT` brings the wait back to `FIRST_RESTART_WAIT`.
    fn watcher_ran_for(&mut self, uptime: Duration) {
        if uptime >= MAX_RESTART_WAIT {
            self.next = FIRST_RESTART_WAIT;
        }
    }

    /// Each call doubles the wait returned by the next one, up to `MAX_RESTART_WAIT`.
    fn next_wait(&mut self) -> Duration {
        let wait = self.next;
        self.next = (wait * 2).min(MAX_RESTART_WAIT);
        wait
    }

    /// A random wait between half of `wait` and `wait`, so nodes that fail together do not retry together.
    fn with_jitter(wait: Duration) -> Duration {
        rand::rng().random_range(wait / 2..=wait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wait_doubles_up_to_the_cap_and_starts_over_after_a_healthy_run() {
        let mut backoff = RestartBackoff::new();
        assert_eq!(backoff.next_wait(), FIRST_RESTART_WAIT);
        assert_eq!(backoff.next_wait(), FIRST_RESTART_WAIT * 2);

        let later_waits: Vec<_> = (0..32).map(|_| backoff.next_wait()).collect();
        assert_eq!(later_waits.last(), Some(&MAX_RESTART_WAIT));
        assert!(later_waits.into_iter().all(|wait| wait <= MAX_RESTART_WAIT));

        backoff.watcher_ran_for(MAX_RESTART_WAIT - Duration::from_secs(1));
        assert_eq!(backoff.next_wait(), MAX_RESTART_WAIT);

        backoff.watcher_ran_for(MAX_RESTART_WAIT);
        assert_eq!(backoff.next_wait(), FIRST_RESTART_WAIT);
    }
}
