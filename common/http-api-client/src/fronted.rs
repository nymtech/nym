//! Utilities for and implementation of request tunneling

use crate::Client;
use crate::ClientBuilder;

use reqwest::header::{HeaderValue, HeaderMap, HOST};
use url::Url;

use std::time::Duration;

impl Client {
	pub fn new_fronted(base_url: Url, fronting_url: Url, timeout: Option<Duration>) -> Self {
        let host = base_url.host_str().unwrap();
        let mut fronted_url = base_url.clone();
        fronted_url.set_host(fronting_url.host_str()).unwrap();
        let builder = ClientBuilder::new::<_, String>(fronted_url)
            .expect(
                "we provided valid url and we were unwrapping previous construction errors anyway",
            )
            .with_host_header(host);

        //SW polish that later if needed
        match timeout {
            Some(timeout) => builder.with_timeout(timeout).build::<String>().unwrap(),
            None => builder.build::<String>().unwrap(),
        }
    }

	pub fn change_fronted_url(&mut self, new_api_url: Url, new_fronting_url: Url) {
        let host = new_api_url.host_str().unwrap();
        let mut new_fronted_url = new_api_url.clone();
        new_fronted_url
            .set_host(new_fronting_url.host_str())
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(host).unwrap()); //SW Handle this unwrap later
        self.reqwest_client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();
        self.base_url = new_fronted_url
    }
}

impl ClientBuilder {
	pub fn with_host_header(mut self, host: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(host).unwrap()); //SW Handle this unwrap later
        self.reqwest_client_builder = self.reqwest_client_builder.default_headers(headers);
        self
    }
}