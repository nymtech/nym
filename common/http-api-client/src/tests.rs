use super::*;

#[test]
fn sanitizing_urls() {
    let base_url: Url = "http://foomp.com".parse().unwrap();

    // works with a full string
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, "/foo//bar/", NO_PARAMS).as_str()
    );

    // (and leading slash doesn't matter)
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, "foo//bar/", NO_PARAMS).as_str()
    );

    // works with 1 segment
    assert_eq!(
        "http://foomp.com/foo",
        sanitize_url(&base_url, &["foo"], NO_PARAMS).as_str()
    );

    // works with 2 segments
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, &["foo", "bar"], NO_PARAMS).as_str()
    );

    // works with leading slash
    assert_eq!(
        "http://foomp.com/foo",
        sanitize_url(&base_url, &["/foo"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, &["/foo", "bar"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, &["foo", "/bar"], NO_PARAMS).as_str()
    );

    // works with trailing slash
    assert_eq!(
        "http://foomp.com/foo",
        sanitize_url(&base_url, &["foo/"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, &["foo/", "bar"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, &["foo", "bar/"], NO_PARAMS).as_str()
    );

    // works with both leading and trailing slash
    assert_eq!(
        "http://foomp.com/foo",
        sanitize_url(&base_url, &["/foo/"], NO_PARAMS).as_str()
    );
    assert_eq!(
        "http://foomp.com/foo/bar",
        sanitize_url(&base_url, &["/foo/", "/bar/"], NO_PARAMS).as_str()
    );

    // adds params
    assert_eq!(
        "http://foomp.com/foo/bar?foomp=baz",
        sanitize_url(&base_url, &["foo", "bar"], &[("foomp", "baz")]).as_str()
    );
    assert_eq!(
        "http://foomp.com/foo/bar?arg1=val1&arg2=val2",
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
        "http://broken.nym.badurl".parse()?,
        "http://example.com/".parse()?,
    ])
    .with_retries(3)
    .build::<HttpClientError>()?;

    let req = client.create_get_request(&["/"], NO_PARAMS).unwrap();
    let resp = client.send::<HttpClientError>(req).await?;

    assert_eq!(resp.status(), 200);

    // check that the url was updated
    assert_eq!(client.current_url().as_str(), "http://example.com/");

    Ok(())
}

#[test]
fn host_updating() {
    let url = Url::new("http://example.com", None).unwrap();
    let mut client = ClientBuilder::new::<_, &str>(url)
        .unwrap()
        .build::<&str>()
        .unwrap();

    // check that the url is set correctly
    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://example.com/");
    assert_eq!(current_url.front_str(), None);

    // update the url
    client.update_host();

    // check that the url is still the same since there is one URL
    assert_eq!(client.current_url().as_str(), "http://example.com/");

    // =======================================
    // we rotate through urls when available

    let new_urls = vec![
        Url::new("http://example.com", None).unwrap(),
        Url::new("http://example.org", None).unwrap(),
    ];
    client.change_base_urls(new_urls);
    assert_eq!(client.current_url().as_str(), "http://example.com/");

    client.update_host();

    // check that the url got updated now that there are multiple URLs
    assert_eq!(client.current_url().as_str(), "http://example.org/");
    assert_eq!(client.current_url().front_str(), None);

    client.update_host();
    assert_eq!(client.current_url().as_str(), "http://example.com/");

    // =======================================
    // we rotate through urls when available if fronting is disabled

    let new_urls = vec![
        Url::new(
            "http://example.com",
            Some(vec!["http://front1.com", "http://front2.com"]),
        )
        .unwrap(),
        Url::new("http://example.org", None).unwrap(),
    ];
    client.change_base_urls(new_urls);

    assert_eq!(client.current_url().as_str(), "http://example.com/");

    client.update_host();

    // check that the url got updated now that there are multiple URLs
    assert_eq!(client.current_url().as_str(), "http://example.org/");
}

#[test]
#[cfg(feature = "tunneling")]
fn fronted_host_updating() {
    let url = Url::new("http://example.com", Some(vec!["http://front1.com"])).unwrap();
    let mut client = ClientBuilder::new::<_, &str>(url)
        .unwrap()
        .with_fronting(crate::fronted::FrontPolicy::Always)
        .build::<&str>()
        .unwrap();

    // check that the url is set correctly
    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://example.com/");
    assert_eq!(current_url.front_str(), Some("front1.com"));

    // update the url
    client.update_host();

    // check that the url is still the same since there is one URL and one front
    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://example.com/");
    assert_eq!(current_url.front_str(), Some("front1.com"));

    // =======================================
    // we rotate through front urls when available if fronting is enabled

    let new_urls = vec![
        Url::new(
            "http://example.com",
            Some(vec!["http://front1.com", "http://front2.com"]),
        )
        .unwrap(),
        Url::new("http://example.org", None).unwrap(),
    ];
    client.change_base_urls(new_urls);

    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://example.com/");
    assert_eq!(current_url.front_str(), Some("front1.com"));

    // update the url - this should keep the same host but change the front
    client.update_host();

    let current_url = client.current_url();
    // check that the url is still the same since there is one URL
    assert_eq!(current_url.as_str(), "http://example.com/");
    assert_eq!(current_url.front_str(), Some("front2.com"));

    // update the url - this should wrap around to the first front as the second url is not fronted
    client.update_host();

    let current_url = client.current_url();
    assert_eq!(current_url.as_str(), "http://example.com/");
    assert_eq!(current_url.front_str(), Some("front1.com"));
}
