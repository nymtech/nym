use super::*;

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
async fn api_client_retry() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new_with_urls(vec![
        "http://broken.nym.test".parse()?, // This should fail because of DNS NXDomain (rotate)
        "http://127.0.0.1:9".parse()?,     // This will fail because of TCP refused (rotate)
        "https://httpbin.org/status/200".parse()?, // This should succeed
    ])?
    .with_retries(3)
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

#[test]
#[cfg(feature = "tunneling")]
fn fronted_host_updating() {
    let url = Url::new("http://nym-api.test", Some(vec!["http://cdn1.test"])).unwrap();
    let mut client = ClientBuilder::new(url)
        .unwrap()
        .with_fronting(crate::fronted::FrontPolicy::Always)
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

#[test]
#[cfg(feature = "network-defaults")]
fn from_network_configures_multiple_urls_and_retries() {
    use nym_network_defaults::{ApiUrl, NymNetworkDetails};

    // Create network details with multiple URLs and fronting
    let mut network_details = NymNetworkDetails::new_empty();
    network_details.nym_api_urls = Some(vec![
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
    let client = ClientBuilder::new_with_fronted_urls(
        network_details.nym_api_urls.clone().unwrap_or_default(),
    )
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
