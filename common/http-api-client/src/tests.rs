use super::*;
use http::{HeaderValue, header::RETRY_AFTER};
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
    assert_eq!(current_url.front_str(), None);

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
    assert_eq!(client.current_url().front_str(), None);

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
    assert_eq!(client.current_url().front_str(), None);

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
    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://nym-api.test/");
    assert_eq!(current_url.front_str(), Some("cdn1.test"));

    // update the url
    client.update_host(None);

    // check that the url is still the same since there is one URL and one front
    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://nym-api.test/");
    assert_eq!(current_url.front_str(), Some("cdn1.test"));

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

    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://nym-api.test/");
    assert_eq!(current_url.front_str(), Some("cdn1.test"));

    // update the url - this should keep the same host but change the front
    client.update_host(None);

    let current_url = client.current_url();
    // check that the url is still the same since there is one URL
    assert_eq!(current_url.as_str(), "http://nym-api.test/");
    assert_eq!(current_url.front_str(), Some("cdn2.test"));

    // update the url - this should wrap around to the first front as the second url is not fronted
    client.update_host(None);

    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://nym-api.test/");
    assert_eq!(current_url.front_str(), Some("cdn1.test"));
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
    assert!(client.base_urls()[0].front_str().is_none());

    assert_eq!(
        client.base_urls()[1].as_str(),
        "https://nym-frontdoor.vercel.app/api/"
    );
    assert!(client.base_urls()[1].front_str().is_some());

    assert_eq!(
        client.base_urls()[2].as_str(),
        "https://nym-frontdoor.global.ssl.fastly.net/api/"
    );
    assert!(client.base_urls()[2].front_str().is_some());
}

/// Tests that network reconfiguration timestamp tempers host rotation / fronting activation.
///
/// If a network reconfiguration happened after request start we avoid rotating and avoid enabling
/// fronting. Otherwise, a network error should rotate host and enable fronting (for `OnRetry`).
#[tokio::test]
#[cfg(feature = "tunneling")]
#[allow(clippy::await_holding_lock)] // guard is only ever held on the single-threaded test runtime
async fn host_rotation_tempered_by_net_reconfigure() {
    // this test mutates the process-wide SHARED_NETWORK_RECONFIGURATION marker, which would
    // otherwise leak into any other test that sends real requests if run concurrently.
    let _guard = crate::lock_shared_test_state();

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
