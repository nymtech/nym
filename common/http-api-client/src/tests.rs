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

// A plain address and a fronted address take turns: a failure moves the pool that was used to its
// next address and hands the next request to the other pool.
#[test]
#[cfg(feature = "tunneling")]
fn on_retry_alternates_between_the_plain_and_the_fronted_pool() {
    let plain_url = Url::new("https://a.test/api/", None).unwrap();
    let fronted_url = Url::new(
        "https://b.test/api/",
        Some(vec!["https://f1.test", "https://f2.test"]),
    )
    .unwrap();
    let client = ClientBuilder::new_with_urls(vec![plain_url, fronted_url])
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::OnRetry))
        .build()
        .unwrap();
    let fail = |address: &str| client.maybe_rotate_hosts(Some(Url::parse(address).unwrap()));

    assert_eq!(client.current_url_str(), "https://a.test/api/");

    fail("https://a.test/api/");
    assert_eq!(client.current_url_str(), "https://f1.test/");
    assert_eq!(client.current_url().as_str(), "https://b.test/api/");

    fail("https://f1.test/");
    assert_eq!(client.current_url_str(), "https://b.test/api/");

    fail("https://b.test/api/");
    assert_eq!(client.current_url_str(), "https://f2.test/");

    fail("https://f2.test/");
    assert_eq!(client.current_url_str(), "https://a.test/api/");
}

// Always uses the fronted pool only, so hosts without a front are not part of it.
#[test]
#[cfg(feature = "tunneling")]
fn always_uses_only_the_fronted_pool() {
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

    assert_eq!(
        client.current_front_host(),
        Some("yelp.global.ssl.fastly.net")
    );
    assert_eq!(
        client.current_url().as_str(),
        "https://nymvpn-frontdoor.global.ssl.fastly.net/api"
    );
}

// With one host and one front, the retry after a failed plain request goes through the front.
#[tokio::test]
#[serial]
#[cfg(feature = "tunneling")]
async fn on_retry_reaches_the_front_on_the_first_retry_with_one_host() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // the plain address is dead, the front answers
    let dead = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
    let port = dead.local_addr().unwrap().port();
    drop(dead);
    let front = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = front.accept().await.unwrap();
        let _ = socket.read(&mut [0u8; 1024]).await;
        let _ = socket
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .await;
    });
    let client = ClientBuilder::new(
        Url::new(
            format!("http://[::1]:{port}"),
            Some(vec![format!("http://127.0.0.1:{port}")]),
        )
        .unwrap(),
    )
    .unwrap()
    .with_fronting(Some(crate::fronted::FrontPolicy::OnRetry))
    .with_retries(1)
    .build()
    .unwrap();

    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();

    assert!(client.send(req).await.unwrap().status().is_success());
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

    // Simulate no recent network reconfiguration. Now the same network error should move the
    // plain pool to the next host and send the next request through the fronted pool.
    *crate::SHARED_NETWORK_RECONFIGURATION.lock().unwrap() =
        Some(Instant::now() - Duration::from_secs(60));
    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();
    let _ = client.send(req).await;

    assert_eq!(client.current_url().as_str(), "http://nym-api.test/");
    assert_eq!(request_host(&client), "cdn1.test");

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

/// `current_url_str()` must agree with `current_front_host()` about which front is active.
#[test]
#[cfg(feature = "tunneling")]
fn current_url_str_tracks_the_active_front() {
    let url = Url::new(
        "https://a.test",
        Some(vec!["https://f0.test", "https://f1.test"]),
    )
    .unwrap();
    let client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(Some(crate::fronted::FrontPolicy::Always))
        .build()
        .unwrap();

    // rotate onto the second front (f1) via the plain policy.
    client.update_host(None);

    assert_eq!(client.current_front_host(), Some("f1.test"));
    assert_eq!(
        client.current_url_str(),
        "https://f1.test/",
        "current_url_str() disagrees with current_front_host() about the active front"
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

// A host that answers 429, 502, 503 or 504 is skipped for the next one; a 404 is returned.
#[tokio::test]
#[serial]
async fn unavailable_host_is_skipped_but_other_errors_are_returned() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let cases = [
        ("429 Too Many Requests", true),
        ("502 Bad Gateway", true),
        ("503 Service Unavailable", true),
        ("504 Gateway Timeout", true),
        ("404 Not Found", false),
    ];
    for (first_answer, skipped) in cases {
        // rotation swaps only the host, so both hosts share one port
        let first = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
        let port = first.local_addr().unwrap().port();
        let second = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .unwrap();
        for (listener, answer) in [(first, first_answer), (second, "200 OK")] {
            tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let _ = socket.read(&mut [0u8; 1024]).await;
                let reply =
                    format!("HTTP/1.1 {answer}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
        let client = ClientBuilder::new_with_urls(vec![
            Url::new(format!("http://[::1]:{port}"), None).unwrap(),
            Url::new(format!("http://127.0.0.1:{port}"), None).unwrap(),
        ])
        .unwrap()
        .with_retries(1)
        .build()
        .unwrap();

        let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();
        let status = client.send(req).await.unwrap().status();

        assert_eq!(status.is_success(), skipped, "{first_answer}");
    }
}

// With one host there is nowhere to rotate to, so an unavailable answer is returned as is.
#[tokio::test]
#[serial]
async fn single_unavailable_host_is_not_retried() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = socket.read(&mut [0u8; 1024]).await;
        let _ = socket
            .write_all(
                b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            )
            .await;
    });
    let client = ClientBuilder::new_with_urls(vec![
        Url::new(format!("http://127.0.0.1:{port}"), None).unwrap(),
    ])
    .unwrap()
    .with_retries(1)
    .build()
    .unwrap();

    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();

    assert_eq!(
        client.send(req).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

// A plain 500 is retried on the same host, then the next call goes to the next host.
#[tokio::test]
#[serial]
async fn failing_host_gets_the_retries_then_the_next_call_rotates() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let first = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
    let port = first.local_addr().unwrap().port();
    let second = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    for (listener, answer) in [(first, "500 Internal Server Error"), (second, "200 OK")] {
        let hits = hits.clone();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                if answer.starts_with("500") {
                    hits.fetch_add(1, Ordering::SeqCst);
                }
                let _ = socket.read(&mut [0u8; 1024]).await;
                let reply =
                    format!("HTTP/1.1 {answer}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
                let _ = socket.write_all(reply.as_bytes()).await;
            }
        });
    }
    let client = ClientBuilder::new_with_urls(vec![
        Url::new(format!("http://[::1]:{port}"), None).unwrap(),
        Url::new(format!("http://127.0.0.1:{port}"), None).unwrap(),
    ])
    .unwrap()
    .with_retries(2)
    .build()
    .unwrap();

    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();
    let first_call = client.send(req).await.unwrap().status();
    let req = client.create_get_request(&["health"], NO_PARAMS).unwrap();
    let second_call = client.send(req).await.unwrap().status();

    assert_eq!(first_call, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    assert!(second_call.is_success());
}

// A request that is not safe to send twice is not resent after a gateway error.
#[tokio::test]
#[serial]
async fn post_answered_with_a_gateway_error_is_not_resent() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let first = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
    let port = first.local_addr().unwrap().port();
    let second = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    for (listener, answer) in [(first, "502 Bad Gateway"), (second, "200 OK")] {
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = socket.read(&mut [0u8; 1024]).await;
            let reply =
                format!("HTTP/1.1 {answer}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
            let _ = socket.write_all(reply.as_bytes()).await;
        });
    }
    let client = ClientBuilder::new_with_urls(vec![
        Url::new(format!("http://[::1]:{port}"), None).unwrap(),
        Url::new(format!("http://127.0.0.1:{port}"), None).unwrap(),
    ])
    .unwrap()
    .with_retries(1)
    .build()
    .unwrap();

    let req = client
        .create_request(reqwest::Method::POST, &["x"], NO_PARAMS, None::<&()>)
        .unwrap();

    assert_eq!(
        client.send(req).await.unwrap().status(),
        StatusCode::BAD_GATEWAY
    );
}

// A request sent before the pool made a full lap must not move the pool when it fails later.
#[test]
fn failure_of_a_request_sent_before_a_full_lap_is_ignored() {
    let client = ClientBuilder::new_with_urls(vec![
        Url::new("http://nym-api1.test", None).unwrap(),
        Url::new("http://nym-api2.test", None).unwrap(),
    ])
    .unwrap()
    .build()
    .unwrap();
    let stale = client.target();

    client.update_host(None);
    client.update_host(None);
    client.shift(&stale, true);

    assert_eq!(client.current_url().as_str(), "http://nym-api1.test/");
}

// A server error moves the pool on, but only interference-like failures hand over to the other pool.
#[test]
#[cfg(feature = "tunneling")]
fn server_error_shifts_within_the_pool_without_switching_to_the_fronted_pool() {
    let client = ClientBuilder::new_with_urls(vec![
        Url::new("https://a.test/api/", Some(vec!["https://f1.test"])).unwrap(),
        Url::new("https://b.test/api/", Some(vec!["https://f2.test"])).unwrap(),
    ])
    .unwrap()
    .with_fronting(Some(crate::fronted::FrontPolicy::OnRetry))
    .build()
    .unwrap();

    client.shift(&client.target(), false);
    assert_eq!(client.current_url_str(), "https://b.test/api/");

    client.shift(&client.target(), true);
    assert_eq!(client.current_url_str(), "https://f1.test/");
}

// A POST is resent when its connection never got established, but not when the server may have
// received it.
#[tokio::test]
#[serial]
async fn post_is_resent_only_when_the_connection_failed() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    for server_received_it in [false, true] {
        let first = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
        let port = first.local_addr().unwrap().port();
        let second = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .unwrap();
        let received = Arc::new(AtomicUsize::new(0));
        if server_received_it {
            // reads each request, then closes the connection without answering
            let received = received.clone();
            tokio::spawn(async move {
                loop {
                    let (mut socket, _) = first.accept().await.unwrap();
                    received.fetch_add(1, Ordering::SeqCst);
                    let _ = socket.read(&mut [0u8; 1024]).await;
                }
            });
        } else {
            drop(first);
        }
        tokio::spawn(async move {
            let (mut socket, _) = second.accept().await.unwrap();
            let _ = socket.read(&mut [0u8; 1024]).await;
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .await;
        });
        let client = ClientBuilder::new_with_urls(vec![
            Url::new(format!("http://[::1]:{port}"), None).unwrap(),
            Url::new(format!("http://127.0.0.1:{port}"), None).unwrap(),
        ])
        .unwrap()
        .with_retries(1)
        .build()
        .unwrap();

        let req = client
            .create_request(reqwest::Method::POST, &["x"], NO_PARAMS, None::<&()>)
            .unwrap();
        let result = client.send(req).await;

        assert_eq!(result.is_ok(), !server_received_it);
        if server_received_it {
            assert_eq!(received.load(Ordering::SeqCst), 1, "the POST was resent");
        }
    }
}
