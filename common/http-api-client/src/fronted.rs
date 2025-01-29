//! Utilities for and implementation of request tunneling

use crate::Client;
use crate::ClientBuilder;

use reqwest::header::{HeaderMap, HeaderValue, HOST};
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Front {
    pub(crate) opts: FrontOptions,
    pub(crate) front: Vec<Url>,

    current_front_idx: usize,
    next_front_idx: usize,
}

#[derive(Debug, PartialEq, Clone)]
struct FrontOptions {
    policy: FrontPolicy,
    retries: usize
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum FrontPolicy {
    RoundRobin,
}

impl Client {
    pub fn new_fronted(base_url: Url, fronts: Vec<Url>) -> Self {
        let host = base_url.host_str().unwrap();
        let mut fronted_url = base_url.clone();
        fronted_url.set_host(fronting_url.host_str()).unwrap();
        let builder = ClientBuilder::new::<_, String>(fronted_url)
            .expect(
                "we provided valid url and we were unwrapping previous construction errors anyway",
            )
            .with_host_header(host);
    }

    // pub fn update_urls(&mut self, new_api_url: Url, new_fronts: Vec<Url>) {
    //     let host = new_api_url.host_str().unwrap();

    //     let fronts = 

    //     let mut new_fronted_url = new_api_url.clone();
    //     new_fronted_url
    //         .set_host(new_fronting_url.host_str())
    //         .unwrap();
    //     let mut headers = HeaderMap::new();
    //     headers.insert(HOST, HeaderValue::from_str(host).unwrap()); //SW Handle this unwrap later
    //     self.reqwest_client = reqwest::ClientBuilder::new()
    //         .default_headers(headers)
    //         .build()
    //         .unwrap();
    //     self.base_url = new_fronted_url
    // }

    pub fn create_request<B, K, V>(
        &self,
        method: reqwest::Method,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: Option<&B>,
    ) -> RequestBuilder
    where
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {}

    async fn send_request<B, K, V, E>(
        &self,
        method: reqwest::Method,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: Option<&B>,
    ) -> Result<Response, HttpClientError<E>>
    where
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
        E: Display,
    {}
}

impl ClientBuilder {
    pub fn with_host_header(mut self, host: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(host).unwrap()); //SW Handle this unwrap later
        self.reqwest_client_builder = self.reqwest_client_builder.default_headers(headers);
        self
    }
}
