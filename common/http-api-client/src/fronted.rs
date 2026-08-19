// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Utilities for and implementation of request tunneling

use std::collections::HashMap;
use std::sync::{
    Arc, LazyLock, Mutex, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tracing::warn;

use crate::{Client, ClientBuilder};

static SHARED_FRONTING_POLICY: LazyLock<Arc<RwLock<FrontPolicy>>> =
    LazyLock::new(|| Arc::new(RwLock::new(FrontPolicy::Off)));

// #[cfg(feature = "tunneling")]
#[derive(Debug)]
pub(crate) struct Front {
    pub(crate) policy: Arc<RwLock<FrontPolicy>>,
    enabled: AtomicBool,
    // Per-domain failure counts, used by `FrontPolicy::ConfiguredRetry` to decide when enough
    // domains have failed often enough to justify enabling fronting.
    domain_failures: Mutex<HashMap<String, usize>>,
}

impl Clone for Front {
    fn clone(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            enabled: AtomicBool::new(false),
            domain_failures: Mutex::new(HashMap::new()),
        }
    }
}

impl Front {
    pub(crate) fn new(policy: FrontPolicy) -> Self {
        Self {
            enabled: AtomicBool::new(false),
            policy: Arc::new(RwLock::new(policy)),
            domain_failures: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn off() -> Self {
        Self::new(FrontPolicy::Off)
    }

    pub(crate) fn shared() -> Self {
        let policy = SHARED_FRONTING_POLICY.clone();
        Self {
            enabled: AtomicBool::new(false),
            policy,
            domain_failures: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn set_policy(&self, policy: FrontPolicy) {
        *self.policy.write().unwrap() = policy;
        self.enabled.store(false, Ordering::Relaxed);
        self.domain_failures.lock().unwrap().clear();
    }

    pub(crate) fn is_enabled(&self) -> bool {
        match *self.policy.read().unwrap() {
            FrontPolicy::Off => false,
            FrontPolicy::OnRetry | FrontPolicy::ConfiguredRetry(_) => {
                self.enabled.load(Ordering::Relaxed)
            }
            FrontPolicy::Always => true,
        }
    }

    // Used to indicate that the client hit an error for the given domain that should trigger the
    // retry policy to enable fronting. `domain` should be the host of the URL that failed.
    pub(crate) fn retry_enable(&self, domain: Option<&str>) {
        if self.is_enabled() {
            return;
        }

        match &*self.policy.read().unwrap() {
            FrontPolicy::OnRetry => self.enabled.store(true, Ordering::Relaxed),
            FrontPolicy::ConfiguredRetry(cfg) => {
                let Some(domain) = domain else {
                    return;
                };

                let mut failures = self.domain_failures.lock().unwrap();
                let count = failures.entry(domain.to_string()).or_insert(0);
                *count += 1;

                let failed_domains = failures
                    .values()
                    .filter(|&&count| count >= cfg.failures_per_domain)
                    .count();

                if failed_domains >= cfg.num_domains_failed {
                    self.enabled.store(true, Ordering::Relaxed);
                }
            }
            FrontPolicy::Off | FrontPolicy::Always => {}
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
/// Policy for when to use domain fronting for HTTP requests.
pub enum FrontPolicy {
    /// Always use domain fronting for all requests.
    Always,

    /// Only use domain fronting when retrying failed requests.
    OnRetry,

    /// Only enable domain fronting once enough distinct domains have each failed enough times,
    /// as configured by [`FrontingConfig`].
    ConfiguredRetry(FrontingConfig),

    /// Never use domain fronting.
    #[default]
    Off,
}

/// Configuration for [`FrontPolicy::ConfiguredRetry`]. A domain is considered "failed" once it
/// has failed `failures_per_domain` times; fronting is enabled once `num_domains_failed` distinct
/// domains have failed.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct FrontingConfig {
    /// Allow N failures per domain before fronting enables
    failures_per_domain: usize,

    /// Allow N domains to fail before fronting is enabled
    num_domains_failed: usize,
}

impl FrontingConfig {
    /// Create a new configuration for [`FrontPolicy::ConfiguredRetry`].
    pub fn new(failures_per_domain: usize, num_domains_failed: usize) -> Self {
        Self {
            failures_per_domain,
            num_domains_failed,
        }
    }

    /// Set the number of failures allowed for a single domain before it is considered failed.
    pub fn with_failures_per_domain(mut self, failures_per_domain: usize) -> Self {
        self.failures_per_domain = failures_per_domain;
        self
    }

    /// Set the number of distinct domains that must fail before fronting is enabled.
    pub fn with_num_domains_failed(mut self, num_domains_failed: usize) -> Self {
        self.num_domains_failed = num_domains_failed;
        self
    }
}

impl ClientBuilder {
    /// Enable and configure request tunneling for API requests. If no front policy is
    /// provided the shared fronting policy will be used.
    pub fn with_fronting(mut self, policy: Option<FrontPolicy>) -> Self {
        let front = if let Some(p) = policy {
            Front::new(p)
        } else {
            Front::shared()
        };

        // Check if any of the supplied urls even support fronting
        if !self.urls.iter().any(|url| url.has_front()) {
            warn!(
                "fronting is enabled, but none of the supplied urls have configured fronting domains: {:?}",
                self.urls
            );
        }

        self.front = front;

        self
    }
}

impl Client {
    /// Set the policy for enabling fronting. If fronting was previously unset this will set it, and
    /// make it possible to enable (i.e [`FrontPolicy::Off`] will not enable it).
    ///
    /// Calling this function sets a custom policy for this client, disconnecting it from the shared
    /// fronting policy -- i.e. changes applied through [`Client::set_shared_front_policy`] will not
    /// be impact this client.
    pub fn set_front_policy(&mut self, policy: FrontPolicy) {
        self.front.set_policy(policy)
    }

    /// Set the fronting policy for this client to follow the shared policy.
    pub fn use_shared_front_policy(&mut self) {
        self.front = Front::shared();
    }

    /// Set the fronting policy for all clients using the shared policy.
    //
    // NOTE: this does not reset the per-instance enabled flag like it will when using
    // [`Front::set_front_policy`]. So if a client is using shared policy with the `OnRetry` policy
    // and this function is used to swap that policy away from and then back to `OnRetry` the
    // fronting will still be enabled. Noting this here just in case this triggers any corner cases
    // down the road.
    pub fn set_shared_front_policy(policy: FrontPolicy) {
        *SHARED_FRONTING_POLICY.write().unwrap() = policy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApiClientCore, NO_PARAMS, Url};

    impl Front {
        pub(crate) fn policy(&self) -> FrontPolicy {
            self.policy.read().unwrap().clone()
        }
    }

    /// Policy can be set for an independent client and the update is applied properly
    #[test]
    fn set_policy_independent_client() {
        let url1 = Url::new(
            "https://validator.global.ssl.fastly.net",
            Some(vec!["https://yelp.global.ssl.fastly.net"]),
        )
        .unwrap();

        let mut client1 = ClientBuilder::new(url1.clone())
            .unwrap()
            .with_fronting(Some(FrontPolicy::Off))
            .build()
            .unwrap();
        assert!(client1.front.policy() == FrontPolicy::Off);

        let client2 = ClientBuilder::new(url1.clone())
            .unwrap()
            .with_fronting(Some(FrontPolicy::OnRetry))
            .build()
            .unwrap();

        // Ensure that setting the policy for a client it gets properly applied.
        client1.set_front_policy(FrontPolicy::Always);
        assert!(client1.front.policy() == FrontPolicy::Always);

        // ensure that setting the policy in a client NOT using the shared policy does NOT update
        // the policy used by another client.
        assert!(client2.front.policy() == FrontPolicy::OnRetry);

        // Ensure that the policy takes effect and is applied when setting host headers on outgoing
        // requests
        let req = client1
            .create_request(reqwest::Method::GET, &["/"], NO_PARAMS, None::<&()>)
            .unwrap()
            .build()
            .unwrap();

        let expected_host = url1.host_str().unwrap();
        assert!(
            req.headers()
                .get(reqwest::header::HOST)
                .is_some_and(|h| h.to_str().unwrap() == expected_host),
            "{:?} != {:?}",
            expected_host,
            req,
        );

        let expected_front = url1.front_str().unwrap();
        assert!(
            req.url()
                .host()
                .is_some_and(|url| url.to_string() == expected_front),
            "{:?} != {:?}",
            expected_front,
            req,
        );
    }

    /// `ConfiguredRetry` should only enable fronting once enough distinct domains have each
    /// failed at least `failures_per_domain` times.
    #[test]
    fn configured_retry_enables_after_thresholds_met() {
        let cfg = FrontingConfig::new(2, 2);
        let front = Front::new(FrontPolicy::ConfiguredRetry(cfg));
        assert!(!front.is_enabled());

        // one failure on domain-a is not enough to count it as "failed" (needs 2)
        front.retry_enable(Some("domain-a"));
        assert!(!front.is_enabled());

        // domain-a fails again, reaching its threshold, but only 1 domain has failed so far
        // (needs 2 domains)
        front.retry_enable(Some("domain-a"));
        assert!(!front.is_enabled());

        // repeated failures on the SAME domain don't count as additional failed domains
        front.retry_enable(Some("domain-a"));
        assert!(!front.is_enabled());

        // domain-b fails once - not enough on its own
        front.retry_enable(Some("domain-b"));
        assert!(!front.is_enabled());

        // domain-b reaches its own threshold - now 2 distinct domains have failed enough times
        front.retry_enable(Some("domain-b"));
        assert!(front.is_enabled());
    }

    /// `ConfiguredRetry` should ignore failures with no associated domain rather than panicking
    /// or enabling fronting.
    #[test]
    fn configured_retry_ignores_failures_without_a_domain() {
        let cfg = FrontingConfig::new(1, 1);
        let front = Front::new(FrontPolicy::ConfiguredRetry(cfg));

        front.retry_enable(None);
        assert!(!front.is_enabled());
    }

    /// Setting a new policy resets both the enabled flag and any accumulated per-domain failure
    /// counts.
    #[test]
    fn configured_retry_resets_on_policy_change() {
        let cfg = FrontingConfig::new(1, 1);
        let front = Front::new(FrontPolicy::ConfiguredRetry(cfg.clone()));

        front.retry_enable(Some("domain-a"));
        assert!(front.is_enabled());

        // re-applying the same policy should reset accumulated failure state
        front.set_policy(FrontPolicy::ConfiguredRetry(cfg));
        assert!(!front.is_enabled());
    }

    /// Policy can be set for the shared client and the update is applied properly.
    ///
    /// This mutates the process-wide `SHARED_FRONTING_POLICY`, so it holds the shared test-state
    /// lock to prevent interference with other tests (previously disabled for this reason).
    #[test]
    fn set_policy_shared_client() {
        let _guard = crate::lock_shared_test_state();

        let url1 = Url::new(
            "https://validator.global.ssl.fastly.net",
            Some(vec!["https://yelp.global.ssl.fastly.net"]),
        )
        .unwrap();

        Client::set_shared_front_policy(FrontPolicy::Off);
        assert!(*SHARED_FRONTING_POLICY.read().unwrap() == FrontPolicy::Off);

        let client1 = ClientBuilder::new(url1.clone())
            .unwrap()
            .with_fronting(None)
            .build()
            .unwrap();
        assert!(client1.front.policy() == FrontPolicy::Off);

        let mut client2 = ClientBuilder::new(url1.clone())
            .unwrap()
            .with_fronting(Some(FrontPolicy::Off))
            .build()
            .unwrap();

        // Ensure that setting the shared policy gets properly applied
        Client::set_shared_front_policy(FrontPolicy::Always);
        assert!(client1.front.policy() == FrontPolicy::Always);

        // Setting the shared policy should NOT update clients NOT using the shared policy.
        assert!(client2.front.policy() == FrontPolicy::Off);

        // Ensure that the policy takes effect and is applied when setting host headers on outgoing
        // requests
        let req = client1
            .create_request(reqwest::Method::GET, &["/"], NO_PARAMS, None::<&()>)
            .unwrap()
            .build()
            .unwrap();

        let expected_host = url1.host_str().unwrap();
        assert!(
            req.headers()
                .get(reqwest::header::HOST)
                .is_some_and(|h| h.to_str().unwrap() == expected_host),
            "{:?} != {:?}",
            expected_host,
            req,
        );

        let expected_front = url1.front_str().unwrap();
        assert!(
            req.url()
                .host()
                .is_some_and(|url| url.to_string() == expected_front),
            "{:?} != {:?}",
            expected_front,
            req,
        );

        // ensure that setting to the shared policy works
        client2.use_shared_front_policy();
        assert!(client2.front.policy() == FrontPolicy::Always);

        // ensure that if the policy is OnRetry then the `enabled` fields are still independent,
        // despite the policy being shared.
        Client::set_shared_front_policy(FrontPolicy::OnRetry);
        assert!(client1.front.policy() == FrontPolicy::OnRetry);
        assert!(client2.front.policy() == FrontPolicy::OnRetry);

        assert!(!client1.front.is_enabled());
        assert!(!client2.front.is_enabled());

        client1.front.retry_enable(None);
        assert!(client1.front.is_enabled());
        assert!(!client2.front.is_enabled());

        // leave the shared policy as we found it for whichever test runs next
        Client::set_shared_front_policy(FrontPolicy::Off);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // guard is only ever held on the single-threaded test runtime
    async fn nym_api_works() {
        // sends a real request, which reads the process-wide SHARED_NETWORK_RECONFIGURATION
        // marker - must not run concurrently with tests that mutate it.
        let _guard = crate::lock_shared_test_state();

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
            .with_fronting(Some(FrontPolicy::Always))
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
    #[allow(clippy::await_holding_lock)] // guard is only ever held on the single-threaded test runtime
    async fn fallback_on_failure() {
        // sends real requests and relies on host rotation not being suppressed, which depends on
        // the process-wide SHARED_NETWORK_RECONFIGURATION marker - must not run concurrently with
        // tests that mutate it.
        let _guard = crate::lock_shared_test_state();

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
            .with_fronting(Some(FrontPolicy::Always))
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
