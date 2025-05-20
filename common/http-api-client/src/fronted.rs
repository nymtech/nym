// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Utilities for and implementation of request tunneling

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::ClientBuilder;

use url::Url;

// #[cfg(feature = "tunneling")]
#[derive(Debug)]
pub(crate) struct Front {
    pub(crate) opts: FrontOptions,
    pub(crate) fronts: Vec<Url>,

    current_front_idx: AtomicUsize,
    next_front_idx: AtomicUsize,
    enabled: AtomicBool,
}

#[cfg(feature = "tunneling")]
impl Clone for Front {
    fn clone(&self) -> Self {
        Self {
            opts: self.opts.clone(),
            fronts: self.fronts.clone(),
            current_front_idx: AtomicUsize::new(self.current_front_idx.load(Ordering::Relaxed)),
            next_front_idx: AtomicUsize::new(self.next_front_idx.load(Ordering::Relaxed)),
            enabled: AtomicBool::new(self.enabled.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(feature = "tunneling")]
impl Front {
    #[cfg(feature = "tunneling")]
    pub(crate) fn host_str(&self) -> Option<&str> {
        self.fronts
            .get(self.current_front_idx.load(Ordering::Relaxed))
            .and_then(|url| url.host_str())
    }

    #[cfg(feature = "tunneling")]
    pub(crate) fn is_enabled(&self) -> bool {
        match self.opts.policy {
            FrontPolicy::Off => false,
            FrontPolicy::OnRetry => self.enabled.load(Ordering::Relaxed),
            FrontPolicy::Always => true,
        }
    }

    // Used to indicate that the client hit an error that should trigger the retry policy
    // to enable fronting.
    pub(crate) fn retry_enable(&self) {
        if self.is_enabled() {
            return;
        }
        if matches!(self.opts.policy, FrontPolicy::OnRetry) {
            self.enabled.store(true, Ordering::Relaxed);
        }
    }

    #[cfg(feature = "tunneling")]
    pub(crate) fn update_front(&self) {
        match self.opts.strategy {
            FrontUrlStrategy::RoundRobin => {
                let current = self.next_front_idx.load(Ordering::Relaxed);
                self.current_front_idx.store(current, Ordering::Relaxed);
                let next = current + 1 % self.fronts.len();
                self.next_front_idx.store(next, Ordering::Relaxed);
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FrontOptions {
    pub policy: FrontPolicy,
    pub strategy: FrontUrlStrategy,
}

impl FrontOptions {
    pub fn on_retry() -> Self {
        Self {
            policy: FrontPolicy::OnRetry,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg(feature = "tunneling")]
pub enum FrontPolicy {
    Always,
    OnRetry,
    #[default]
    Off,
}

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg(feature = "tunneling")]
pub enum FrontUrlStrategy {
    #[default]
    RoundRobin,
}

impl ClientBuilder {
    /// Enable and configure request tunneling for API requests.
    #[cfg(feature = "tunneling")]
    pub fn with_fronting(mut self, fronts: Vec<Url>, opts: FrontOptions) -> Self {
        let pre_enable = opts.policy == FrontPolicy::Always;
        let front = Front {
            opts,
            fronts,
            current_front_idx: AtomicUsize::new(0_usize),
            next_front_idx: AtomicUsize::new(0_usize), // Fine to start as 0, as we update it immediately
            enabled: AtomicBool::new(pre_enable),
        };

        front.update_front();

        self.front = Some(front);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApiClientCore, NO_PARAMS};

    #[tokio::test]
    async fn nym_api_works() {
        let opts = FrontOptions::default();
        let fronts = vec!["https://yelp.global.ssl.fastly.net".parse().unwrap()]; // fastly
                                                                                  // let fronts = vec!["https://cdn77.com".parse().unwrap()]; // cdn77

        // let client = ClientBuilder::new::<&str, &str>("https://validator.nymtech.net")
        let client = ClientBuilder::new::<&str, &str>("https://validator.global.ssl.fastly.net")
            .expect("bad url")
            .with_fronting(fronts, opts)
            .build::<&str>()
            .expect("failed to build client");

        let response = client
            .send_request::<(), &str, &str, &str>(
                reqwest::Method::GET,
                &["api", "v1", "network", "details"],
                NO_PARAMS,
                None,
            )
            .await
            .expect("failed get request");

        // println!("{response:?}");
        assert_eq!(response.status(), 200);
    }
}
