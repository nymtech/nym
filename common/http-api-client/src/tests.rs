use super::*;
use http::{HeaderValue, header::RETRY_AFTER};
use serial_test::serial;
use std::time::{Duration, Instant};

#[test]
fn sanitizing_urls() {
    let base_url: Url = "http://api.test".parse().unwrap();

    // works with a full string
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, "/foo//bar/", NO_PARAMS).as_str()
    );

    // (and leading slash doesn't matter)
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, "foo//bar/", NO_PARAMS).as_str()
    );

    // works with 1 segment
    assert_eq!(
        "http://api.test/foo",
        sanitize_url(&base_url, &["foo"], NO_PARAMS).as_str()
    );

    // works with 2 segments
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, &["foo", "bar"], NO_PARAMS).as_str()
    );

    // works with leading slash
    assert_eq!(
        "http://api.test/foo",
        sanitize_url(&base_url, &["/foo"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, &["/foo", "bar"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, &["foo", "/bar"], NO_PARAMS).as_str()
    );

    // works with trailing slash
    assert_eq!(
        "http://api.test/foo",
        sanitize_url(&base_url, &["foo/"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, &["foo/", "bar"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, &["foo", "bar/"], NO_PARAMS).as_str()
    );

    // works with both leading and trailing slash
    assert_eq!(
        "http://api.test/foo",
        sanitize_url(&base_url, &["/foo/"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://api.test/foo/bar",
        sanitize_url(&base_url, &["/foo/", "/bar/"], NO_PARAMS).as_str()
    );

    // adds params
    assert_eq!(
        "http://api.test/foo/bar?foomp=baz",
        sanitize_url(&base_url, &["foo", "bar"], &[("foomp", "baz")]).as_str()
    );
    assert_eq!(
        "http://api.test/foo/bar?arg1=val1&arg2=val2",
        sanitize_url(
            &base_url,
            &["/foo/", "/bar/"],
            &[("arg1", "val1"), ("arg2", "val2")]
        )
        .as_str()
    );
}

// - Do the retries work
// - Do we use fallback urls on retry if multiple are provided
// - Do we use the next front on retry if multiple are provided
// - If we have more retries than urls, do we wrap back to the first one again
// - on error without retries is where we have multiple urls, is the url updated?

#[tokio::test]
#[cfg(any())] // #[ignore] we run ignore assuming it just means slow in Ci/CD -_-
// test relies on external services being available and behaving in a specific way.
async fn api_client_retry() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new_with_urls(vec![
        "http://broken.nym.test".parse()?, // This should fail because of DNS NXDomain (rotate)
        "http://127.0.0.1:9".parse()?,     // This will fail because of TCP refused (rotate)
        "https://httpbin.org/status/200".parse()?, // This should succeed
    ])?
    .with_retries(2)
    .build()?;

    let req = client.create_get_request(&[], NO_PARAMS).unwrap();
    let _resp = client.send(req).await?;

    // The main test is that we successfully retried and switched to the working URL
    // We accept any response from the working endpoint since external services can be unreliable
    assert_eq!(
        client.current_url().as_str(),
        "https://httpbin.org/status/200"
    );

    // // This assert can be unreliable due to factors beyond our control and beyond the scope of
    // // this test
    // assert_eq!(_resp.status(), StatusCode::OK);

    Ok(())
}

#[test]
fn host_updating() {
    let url = Url::new("http://nym-api1.test", None).unwrap();
    let mut client = ClientBuilder::new(url).unwrap().build().unwrap();

    // check that the url is set correctly
    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://nym-api1.test/");
    assert_eq!(current_url.first_front_str(), None);

    // update the url
    client.update_host(None);

    // check that the url is still the same since there is one URL
    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");

    // =======================================
    // we rotate through urls when available

    let new_urls = vec![
        Url::new("http://nym-api1.test", None).unwrap(),
        Url::new("http://nym-api2.test", None).unwrap(),
    ];
    client.change_base_urls(new_urls);
    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");

    client.update_host(None);

    // check that the url got updated now that there are multiple URLs
    assert_eq!(client.current_url().as_str(), "http://nym-api2.test/");
    assert_eq!(client.current_url().first_front_str(), None);

    client.update_host(None);
    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");

    // =======================================
    // we rotate through urls when available if fronting is disabled

    let new_urls = vec![
        Url::new(
            "http://nym-api1.test",
            Some(vec!["http://cdn1.test", "http://cdn2.test"]),
        )
        .unwrap(),
        Url::new("http://nym-api2.test", None).unwrap(),
    ];
    client.change_base_urls(new_urls);

    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");

    client.update_host(None);

    // check that the url got updated now that there are multiple URLs
    assert_eq!(client.current_url().as_str(), "http://nym-api2.test/");
}

#[test]
fn host_updating_url_conditioned() {
    let url1 = Url::new("http://nym-api1.test", None).unwrap();
    let url2 = Url::new("http://nym-api2.test", None).unwrap();
    let urls = vec![url1.clone(), url2.clone()];
    let client = ClientBuilder::new_with_urls(urls).unwrap().build().unwrap();

    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");

    // Try to update with a URL that does NOT match current - should result in no change
    client.update_host(Some(Url::parse("http://example.com").unwrap()));

    // check that the url did NOT get updated
    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");
    assert_eq!(client.current_url().first_front_str(), None);

    // Try to update with a URL that DOES match current - should result in no change
    client.update_host(Some(url1));
    assert_eq!(client.current_url().as_str(), "http://nym-api2.test/");
}

// Regression test: `apply_hosts_to_req` must read `current_url()` exactly once and derive
// both the returned domain and the host actually applied to the request from that single
// snapshot. If a caller (or `apply_hosts_to_req` itself) read `current_url()` twice, a
// concurrent host rotation interleaved between the two reads could desync the reported
// domain from the host that ends up on the outgoing request.
#[test]
fn apply_hosts_to_req_domain_matches_request_host() {
    let new_urls = vec![
        Url::new("http://nym-api1.test", None).unwrap(),
        Url::new("http://nym-api2.test", None).unwrap(),
    ];
    let client = ClientBuilder::new_with_urls(new_urls)
        .unwrap()
        .build()
        .unwrap();

    for _ in 0..4 {
        let current = client.current_url().clone();
        let mut req = reqwest::Request::new(reqwest::Method::GET, current.clone().into());

        let (domain, front_used) = client.apply_hosts_to_req(&mut req);

        assert_eq!(domain, current.host_str());
        assert_eq!(front_used, None);
        assert_eq!(req.url().host_str(), current.host_str());

        client.update_host(None);
    }
}

#[test]
#[cfg(feature = "tunneling")]
fn apply_hosts_to_req_domain_matches_real_host_when_fronted() {
    let url = Url::new(
        "http://nym-api.test",
        Some(vec!["http://cdn1.test", "http://cdn2.test"]),
    )
    .unwrap();
    let client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::Always))
        .build()
        .unwrap();

    for _ in 0..3 {
        let mut req =
            reqwest::Request::new(reqwest::Method::GET, client.current_url().clone().into());

        let (domain, front_used) = client.apply_hosts_to_req(&mut req);

        // the real (unfronted) host is always reported, regardless of which front is active
        assert_eq!(domain, Some("nym-api.test"));
        assert!(front_used.is_some());

        // the request itself must be routed via the front, with the real host preserved in
        // the HOST header and the front captured in the outer-SNI header
        assert_eq!(req.url().host_str(), front_used);
        assert_eq!(
            req.headers().get(reqwest::header::HOST).unwrap(),
            domain.unwrap()
        );
        assert_eq!(
            req.headers().get(NYM_OUTER_SNI_HEADER).unwrap(),
            front_used.unwrap()
        );

        client.update_host(None);
    }
}

#[test]
#[cfg(feature = "tunneling")]
fn fronted_host_updating() {
    let url = Url::new("http://nym-api.test", Some(vec!["http://cdn1.test"])).unwrap();
    let mut client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::Always))
        .build()
        .unwrap();

    // check that the url is set correctly
    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(client.current_front_host(), Some("cdn1.test"));

    // update the url
    client.update_host(None);

    // check that the url is still the same since there is one URL and one front
    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(client.current_front_host(), Some("cdn1.test"));

    // =======================================
    // we rotate through front urls when available if fronting is enabled

    let new_urls = vec![
        Url::new(
            "http://nym-api.test",
            Some(vec!["http://cdn1.test", "http://cdn2.test"]),
        )
        .unwrap(),
        Url::new("http://nym-api2.test", None).unwrap(),
    ];
    client.change_base_urls(new_urls);

    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(client.current_front_host(), Some("cdn1.test"));

    // update the url - this should keep the same host but change the front
    client.update_host(None);

    // check that the url is still the same since there is one URL
    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(client.current_front_host(), Some("cdn2.test"));

    // update the url - this should wrap around to the first front as the second url is not fronted
    client.update_host(None);

    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(client.current_front_host(), Some("cdn1.test"));
}

// Reproduces the exact url-list shape used for the nymvpn-api config:
//   [{ "url": "https://nymvpn.com/api/" },
//    { "url": "https://nymvpn-frontdoor.global.ssl.fastly.net/api",
//      "fronts": ["yelp.global.ssl.fastly.net"] }]
//
// When fronting is enabled and the CURRENT url is the one with no `fronts`
// configured, `matches_current_host` must consider the fact that the domain
// doesn't have fronts and compare properly. This test ensures the correct
// behavior, rotating to the next (fronted) entry in the list.
#[test]
#[cfg(feature = "tunneling")]
fn fronting_enabled_stuck_on_unfronted_first_url() {
    let plain_url = Url::new("https://nymvpn.com/api/", None).unwrap();
    let fronted_url = Url::new(
        "https://nymvpn-frontdoor.global.ssl.fastly.net/api",
        Some(vec!["https://yelp.global.ssl.fastly.net"]),
    )
    .unwrap();

    let client = ClientBuilder::new_with_urls(vec![plain_url, fronted_url])
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::Always))
        .build()
        .unwrap();

    // client starts on the first, unfronted url.
    assert_eq!(client.current_url().as_str(), "https://nymvpn.com/api/");

    // offending url matches the plain url
    let offending = Url::parse("https://nymvpn.com/api/").unwrap();
    client.maybe_rotate_hosts(Some(offending));

    // should rotate urls.
    assert_eq!(
        client.current_url().as_str(),
        "https://nymvpn-frontdoor.global.ssl.fastly.net/api",
        "client failed to rotate away from the unfronted url after an error"
    );
}

#[test]
#[cfg(feature = "network-defaults")]
fn from_network_configures_multiple_urls_and_retries() {
    use nym_network_defaults::{ApiUrl, NymNetworkDetails};

    // Create network details with multiple URLs and fronting
    let mut network_details = NymNetworkDetails::new_empty();
    network_details.set_nym_api_urls(vec![
        ApiUrl {
            url: "https://validator.nymtech.net/api/".to_string(),
            front_hosts: None,
        },
        ApiUrl {
            url: "https://nym-frontdoor.vercel.app/api/".to_string(),
            front_hosts: Some(vec!["vercel.app".to_string(), "vercel.com".to_string()]),
        },
        ApiUrl {
            url: "https://nym-frontdoor.global.ssl.fastly.net/api/".to_string(),
            front_hosts: Some(vec!["yelp.global.ssl.fastly.net".to_string()]),
        },
    ]);

    // Build client from network details
    let client = ClientBuilder::new_with_fronted_urls(network_details.nym_api_urls())
        .expect("Failed to create client from network")
        .build()
        .expect("Failed to build client");

    // Verify all URLs were configured
    assert_eq!(
        client.base_urls().len(),
        3,
        "Expected 3 URLs to be configured from network details"
    );

    // Verify the URLs have fronting configured where appropriate
    assert_eq!(
        client.base_urls()[0].as_str(),
        "https://validator.nymtech.net/api/"
    );
    assert!(!client.base_urls()[0].has_front());

    assert_eq!(
        client.base_urls()[1].as_str(),
        "https://nym-frontdoor.vercel.app/api/"
    );
    assert!(client.base_urls()[1].has_front());

    assert_eq!(
        client.base_urls()[2].as_str(),
        "https://nym-frontdoor.global.ssl.fastly.net/api/"
    );
    assert!(client.base_urls()[2].has_front());
}

/// Tests that network reconfiguration timestamp tempers host rotation / fronting activation.
///
/// If a network reconfiguration happened after request start we avoid rotating and avoid enabling
/// fronting. Otherwise, a network error should rotate host and enable fronting (for `OnRetry`).
#[tokio::test]
#[serial]
#[cfg(feature = "tunneling")]
async fn host_rotation_tempered_by_net_reconfigure() {
    // mutates the process-wide SHARED_NETWORK_RECONFIGURATION marker and sends real requests
    // sensitive to it - must not run concurrently with tests that touch the same state.

    let url1 = Url::new("http://nym-api.test", Some(vec!["http://cdn1.test"])).unwrap();
    let url2 = Url::new("http://nym-api2.test", Some(vec!["http://cdn2.test"])).unwrap();
    let urls = vec![url1.clone(), url2.clone()];

    let client = ClientBuilder::new_with_urls(urls)
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::OnRetry))
        .build()
        .unwrap();

    let request_host = |client: &Client| {
        client
            .create_get_request(&["health"], NO_PARAMS)
            .unwrap()
            .build()
            .unwrap()
            .url()
            .host_str()
            .unwrap()
            .to_string()
    };

    // fronting starts disabled for OnRetry policy.
    assert_eq!(request_host(&client), "nym-api.test");
    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");

    // Simulate a network reconfiguration happening during the request. This should suppress both
    // host rotation and fronting activation.
    *crate::SHARED_NETWORK_RECONFIGURATION.lock().unwrap() =
        Some(Instant::now() + Duration::from_secs(60));
    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();
    let _ = client.send(req).await;

    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(request_host(&client), "nym-api.test");

    // Simulate no recent network reconfiguration. Now the same network error should rotate to the
    // next host and enable fronting for OnRetry.
    *crate::SHARED_NETWORK_RECONFIGURATION.lock().unwrap() =
        Some(Instant::now() - Duration::from_secs(60));
    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();
    let _ = client.send(req).await;

    assert_eq!(client.current_url().as_str(), "http://nym-api2.test/");
    assert_eq!(request_host(&client), "cdn2.test");

    // leave the shared marker as we found it for whichever test runs next
    *crate::SHARED_NETWORK_RECONFIGURATION.lock().unwrap() = None;
}

#[test]
fn rate_limit_detection_on_plain_429() {
    // a bare 429 with no special headers should be treated as a rate limit response
    assert!(is_rate_limit_response(
        StatusCode::TOO_MANY_REQUESTS,
        &HeaderMap::new()
    ));

    // the status code alone is sufficient - unrelated headers shouldn't change that
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    assert!(is_rate_limit_response(
        StatusCode::TOO_MANY_REQUESTS,
        &headers
    ));
}

#[test]
fn rate_limit_detection_on_throttled_503() {
    // a 503 with Retry-After is treated as throttling, not just an outage
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("120"));
    assert!(is_rate_limit_response(
        StatusCode::SERVICE_UNAVAILABLE,
        &headers
    ));

    // a plain 503 without Retry-After is NOT treated as rate limiting - it may just be down
    assert!(!is_rate_limit_response(
        StatusCode::SERVICE_UNAVAILABLE,
        &HeaderMap::new()
    ));
}

#[test]
fn rate_limit_detection_on_vercel_challenge() {
    let mut headers = HeaderMap::new();
    headers.insert(
        VERCEL_CHALLENGE_HEADER,
        HeaderValue::from_static("challenge"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));

    assert!(is_rate_limit_response(StatusCode::FORBIDDEN, &headers));
}

#[test]
fn rate_limit_detection_ignores_unrelated_responses() {
    // plain 403 without the vercel challenge markers is not a rate limit
    assert!(!is_rate_limit_response(
        StatusCode::FORBIDDEN,
        &HeaderMap::new()
    ));

    // the vercel challenge header alone, without the matching content-type, is not enough
    let mut headers = HeaderMap::new();
    headers.insert(
        VERCEL_CHALLENGE_HEADER,
        HeaderValue::from_static("challenge"),
    );
    assert!(!is_rate_limit_response(StatusCode::FORBIDDEN, &headers));

    // a normal successful response is never a rate limit
    assert!(!is_rate_limit_response(StatusCode::OK, &HeaderMap::new()));
}

/// `current_url_str()`/`current_front_host()` must not report a stale front once fronting has
/// been turned off. The rotation cursors (`rotation_slot`/`current_front`) are only reset within
/// `update_host` while `self.front.is_enabled()`, so a naive reader of those cursors alone would
/// keep reporting whatever front was last selected even after fronting is disabled - both methods
/// must consult `self.front.is_enabled()` themselves rather than trusting the cursor state.
#[test]
#[cfg(feature = "tunneling")]
fn as_str_does_not_report_front_after_fronting_disabled() {
    use crate::fronted::{FrontPolicy, FrontingConfig};

    let url = Url::new("https://a.test", Some(vec!["https://f0.test"])).unwrap();
    let mut client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(FrontPolicy::ConfiguredRetry(
            FrontingConfig::new(1, 1).with_include_non_fronted_in_rotation(true),
        )))
        .build()
        .unwrap();

    client.front.retry_enable(Some("a.test"));
    assert!(client.front.is_enabled());

    // rotate until the host advances off its direct turn and onto a front (rotation_slot != 0)
    client.update_host(None);
    assert_eq!(client.current_url_str(), "https://f0.test/");
    assert_eq!(client.current_front_host(), Some("f0.test"));

    // now turn fronting off - a plain public API call, no race required
    client.set_front_policy(FrontPolicy::Off);
    assert!(!client.front.is_enabled());

    // the request correctly goes out direct, unfronted...
    let mut req = reqwest::Request::new(reqwest::Method::GET, client.current_url().clone().into());
    let (domain, front_used) = client.apply_hosts_to_req(&mut req);
    assert_eq!(req.url().host_str(), Some("a.test"));
    assert_eq!(domain, Some("a.test"));
    assert_eq!(front_used, None);

    // ...and current_url_str()/current_front_host() agree, despite the untouched rotation_slot.
    assert_eq!(
        client.current_url_str(),
        "https://a.test/",
        "current_url_str() must report the direct host once fronting is disabled"
    );
    assert_eq!(
        client.current_front_host(),
        None,
        "current_front_host() must report no active front once fronting is disabled"
    );
}

#[cfg(feature = "tunneling")]
fn rotating_client(fronts: Vec<&str>) -> Client {
    use crate::fronted::{FrontPolicy, FrontingConfig};
    let url = Url::new("https://a.test", Some(fronts)).unwrap();
    let client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(FrontPolicy::ConfiguredRetry(
            FrontingConfig::new(1, 1).with_include_non_fronted_in_rotation(true),
        )))
        .build()
        .unwrap();
    client.front.retry_enable(Some("a.test"));
    assert!(client.front.is_enabled());
    client
}

/// Relating to `include_non_fronted_in_rotation`: `RotationManager::front_str` is normally
/// driven by the plain `current_front` cursor (only advanced by `update()`), which is never
/// touched by this policy - it advances `rotation_slot` via `take_rotation_turn()` instead. This
/// checks `front_str()` still agrees with the front actually on the wire by falling back to
/// `rotation_slot` whenever `take_rotation_turn()` has it actively engaged.
#[test]
#[cfg(feature = "tunneling")]
fn front_str_tracks_the_front_actually_used() {
    let client = rotating_client(vec!["https://f0.test", "https://f1.test"]);

    // advance onto the SECOND front
    for _ in 0..2 {
        client.update_host(None);
    }

    let mut req = reqwest::Request::new(reqwest::Method::GET, client.current_url().clone().into());
    let (_domain, front_used) = client.apply_hosts_to_req(&mut req);

    assert_eq!(front_used, Some("f1.test"), "sanity: request goes via f1");
    // reaches into the private `rotation` field directly, rather than
    // `Client::current_front_host()`, since that goes through `active_rotation_front_str()`
    // instead and so wouldn't exercise `front_str()`'s own fallback at all.
    assert_eq!(
        client.rotation.front_str(0, client.current_url()),
        front_used,
        "front_str() disagrees with the front on the wire"
    );
}

/// Each lap through a host's rotation should visit the direct (unfronted) turn exactly once,
/// then each configured front exactly once, before repeating - never lingering on the direct
/// turn for two consecutive turns.
#[test]
#[cfg(feature = "tunneling")]
fn rotation_visits_each_slot_once_per_lap() {
    let client = rotating_client(vec!["https://f0.test", "https://f1.test"]);

    let mut seq = Vec::new();
    for _ in 0..6 {
        seq.push(client.current_url_str().to_string());
        client.update_host(None);
    }

    assert_eq!(
        seq,
        vec![
            "https://a.test/",
            "https://f0.test/",
            "https://f1.test/",
            "https://a.test/",
            "https://f0.test/",
            "https://f1.test/",
        ],
    );
}

/// If `num_domains_failed` exceeds the number of distinct base urls, the threshold is
/// unreachable and fronting can never engage no matter how many failures are recorded.
/// `with_fronting` now warns about this misconfiguration (mirroring the existing warning for
/// urls with no fronts configured), but the threshold itself is intentionally left unclamped -
/// so this documents that fronting stays off rather than silently misbehaving.
#[test]
#[cfg(feature = "tunneling")]
fn unsatisfiable_num_domains_failed_never_enables_fronting() {
    use crate::fronted::{FrontPolicy, FrontingConfig};

    // one base url, but the policy demands two distinct domains fail
    let url = Url::new("https://a.test", Some(vec!["https://f0.test"])).unwrap();
    let client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(FrontPolicy::ConfiguredRetry(FrontingConfig::new(
            1, 2,
        ))))
        .build()
        .unwrap();

    for _ in 0..100 {
        client.front.retry_enable(Some("a.test"));
    }

    assert!(
        !client.front.is_enabled(),
        "fronting should never engage: only 1 base url but num_domains_failed = 2"
    );
}

#[test]
fn caller_supplied_host_header_is_preserved() {
    let url = Url::new("https://a.test", None).unwrap();
    let client = ClientBuilder::new(url).unwrap().build().unwrap();

    let mut req = client
        .create_request(reqwest::Method::GET, &["x"], NO_PARAMS, None::<&()>)
        .unwrap()
        .header(reqwest::header::HOST, "caller-supplied.test")
        .build()
        .unwrap();

    client.apply_hosts_to_req(&mut req);

    assert_eq!(
        req.headers()
            .get(reqwest::header::HOST)
            .map(|v| v.to_str().unwrap()),
        Some("caller-supplied.test"),
        "caller's Host header was silently dropped",
    );
}

/// A caller-supplied Host header must keep overriding the front's actual-host value across
/// repeated calls to `apply_hosts_to_req` (e.g. retries) -- not just the first one -- and the
/// request must still be routed via the front's SNI.
#[test]
#[cfg(feature = "tunneling")]
fn caller_supplied_host_header_survives_fronted_retries() {
    let url = Url::new("http://nym-api.test", Some(vec!["http://cdn1.test"])).unwrap();
    let client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::Always))
        .build()
        .unwrap();

    let mut req = client
        .create_request(reqwest::Method::GET, &["x"], NO_PARAMS, None::<&()>)
        .unwrap()
        .header(reqwest::header::HOST, "caller-supplied.test")
        .build()
        .unwrap();

    for _ in 0..3 {
        client.apply_hosts_to_req(&mut req);

        assert_eq!(
            req.headers()
                .get(reqwest::header::HOST)
                .map(|v| v.to_str().unwrap()),
            Some("caller-supplied.test"),
            "caller's Host header must survive repeated (e.g. retried) calls",
        );
        // the request is still routed to the front at the network/SNI level, even though the
        // Host header no longer matches what fronting expects.
        assert_eq!(req.url().host_str(), Some("cdn1.test"));
    }
}

/// Without a caller override, a host rotation between two calls to `apply_hosts_to_req` on the
/// *same* request (as happens across retries in `Client::send`) must still be picked up -- the
/// bookkeeping that lets us detect a caller override must not be mistaken for one itself.
#[test]
#[cfg(feature = "tunneling")]
fn host_rotation_is_still_applied_across_repeated_calls_without_override() {
    let url = Url::new(
        "http://nym-api.test",
        Some(vec!["http://cdn1.test", "http://cdn2.test"]),
    )
    .unwrap();
    let client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::Always))
        .build()
        .unwrap();

    let mut req = reqwest::Request::new(reqwest::Method::GET, client.current_url().clone().into());

    let (_, front_used) = client.apply_hosts_to_req(&mut req);
    assert_eq!(front_used, Some("cdn1.test"));
    assert_eq!(
        req.headers()
            .get(reqwest::header::HOST)
            .map(|v| v.to_str().unwrap()),
        Some("nym-api.test")
    );

    client.update_host(None);

    // reuse the same request/headers, simulating a retried send after a rotation.
    let (_, front_used) = client.apply_hosts_to_req(&mut req);
    assert_eq!(front_used, Some("cdn2.test"), "rotation was not picked up");
    assert_eq!(req.url().host_str(), Some("cdn2.test"));
    assert_eq!(
        req.headers()
            .get(reqwest::header::HOST)
            .map(|v| v.to_str().unwrap()),
        Some("nym-api.test")
    );
}
