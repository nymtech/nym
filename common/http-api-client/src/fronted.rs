// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Utilities for and implementation of request tunneling

use std::sync::atomic::{AtomicBool, Ordering};
use tracing::warn;

use crate::ClientBuilder;

// #[cfg(feature = "tunneling")]
#[derive(Debug)]
pub(crate) struct Front {
    pub(crate) policy: FrontPolicy,
    enabled: AtomicBool,
}

impl Clone for Front {
    fn clone(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            enabled: AtomicBool::new(self.enabled.load(Ordering::Relaxed)),
        }
    }
}

impl Front {
    pub(crate) fn new(policy: FrontPolicy) -> Self {
        Self {
            enabled: AtomicBool::new(policy == FrontPolicy::Always),
            policy,
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        match self.policy {
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
        if matches!(self.policy, FrontPolicy::OnRetry) {
            self.enabled.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg(feature = "tunneling")]
/// Policy for when to use domain fronting for HTTP requests.
pub enum FrontPolicy {
    /// Always use domain fronting for all requests.
    Always,
    /// Only use domain fronting when retrying failed requests.
    OnRetry,
    #[default]
    /// Never use domain fronting.
    Off,
}

impl ClientBuilder {
    /// Enable and configure request tunneling for API requests.
    #[cfg(feature = "tunneling")]
    pub fn with_fronting(mut self, policy: FrontPolicy) -> Self {
        let front = Front::new(policy);

        // Check if any of the supplied urls even support fronting
        if !self.urls.iter().any(|url| url.has_front()) {
            warn!(
                "fronting is enabled, but none of the supplied urls have configured fronting domains"
            );
        }

        self.front = Some(front);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApiClientCore, NO_PARAMS, Url};

    #[tokio::test]
    async fn nym_api_works() {
        let url1 = Url::new(
            "https://validator.global.ssl.fastly.net",
            Some(vec!["https://yelp.global.ssl.fastly.net"]),
        )
        .unwrap(); // fastly

        // let url2 = Url::new(
        //     "https://validator.nymtech.net",
        //     Some(vec!["https://cdn77.com"]),
        // ).unwrap(); // cdn77

        let client = ClientBuilder::new(url1)
            .expect("bad url")
            .with_fronting(FrontPolicy::Always)
            .build()
            .expect("failed to build client");

        let response = client
            .send_request::<_, (), &str, &str>(
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

    #[tokio::test]
    async fn fallback_on_failure() {
        let url1 = Url::new(
            "https://fake-domain.nymtech.net",
            Some(vec![
                "https://fake-front-1.nymtech.net",
                "https://fake-front-2.nymtech.net",
            ]),
        )
        .unwrap();
        let url2 = Url::new(
            "https://validator.global.ssl.fastly.net",
            Some(vec!["https://yelp.global.ssl.fastly.net"]),
        )
        .unwrap(); // fastly

        let client = ClientBuilder::new_with_urls(vec![url1, url2])
            .expect("bad url")
            .with_fronting(FrontPolicy::Always)
            .build()
            .expect("failed to build client");

        // Check that the initial configuration has the broken domain and front.
        assert_eq!(
            client.current_url().as_str(),
            "https://fake-domain.nymtech.net/",
        );
        assert_eq!(
            client.current_url().front_str(),
            Some("fake-front-1.nymtech.net"),
        );

        let result = client
            .send_request::<_, (), &str, &str>(
                reqwest::Method::GET,
                &["api", "v1", "network", "details"],
                NO_PARAMS,
                None,
            )
            .await;
        assert!(result.is_err());

        // Check that the host configuration updated the front on error.
        assert_eq!(
            client.current_url().as_str(),
            "https://fake-domain.nymtech.net/",
        );
        assert_eq!(
            client.current_url().front_str(),
            Some("fake-front-2.nymtech.net"),
        );

        let result = client
            .send_request::<_, (), &str, &str>(
                reqwest::Method::GET,
                &["api", "v1", "network", "details"],
                NO_PARAMS,
                None,
            )
            .await;
        assert!(result.is_err());

        // Check that the host configuration updated the domain and front on error.
        assert_eq!(
            client.current_url().as_str(),
            "https://validator.global.ssl.fastly.net/",
        );
        assert_eq!(
            client.current_url().front_str(),
            Some("yelp.global.ssl.fastly.net"),
        );

        let response = client
            .send_request::<_, (), &str, &str>(
                reqwest::Method::GET,
                &["api", "v1", "network", "details"],
                NO_PARAMS,
                None,
            )
            .await
            .expect("failed get request");

        assert_eq!(response.status(), 200);
    }
}
