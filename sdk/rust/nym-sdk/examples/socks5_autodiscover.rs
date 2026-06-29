//! SOCKS5 proxy client that auto-discovers a network requester, optionally
//! pinned to a set of countries.
//!
//! Unlike `socks5.rs`, this takes no provider address. It queries the Nym API
//! for exit gateways advertising a network requester, optionally keeps only
//! those physically located in the requested countries, picks one weighted by
//! performance, and routes an HTTPS request through it.
//!
//! Run with: cargo run --example socks5_autodiscover

use nym_sdk::mixnet::{NetworkRequester, Socks5MixnetClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    nym_bin_common::logging::setup_tracing_logger();

    // Pick how to choose the requester. Three options:
    //   NetworkRequester::any()                         -> any, weighted by performance
    //   NetworkRequester::in_countries(["CH", "DE"])?   -> restricted to those countries
    //   NetworkRequester::exact("address...")?          -> a specific known requester
    let requester = NetworkRequester::in_countries(["CH", "DE"])?;

    // Connect. `None` binds the SOCKS5 listener to 127.0.0.1:1080; here we pass
    // 1081 to avoid colliding with any proxy already on the default port.
    println!("Discovering a network requester in CH/DE and connecting");
    let bind = Some("127.0.0.1:1081".parse()?);
    let client = Socks5MixnetClient::connect_with(requester, bind).await?;
    println!("SOCKS5 proxy listening at {}", client.socks5_url());

    // Point an HTTP client at the proxy and make a request through the mixnet.
    let proxy = reqwest::Proxy::all(client.socks5_url())?;
    let http = reqwest::Client::builder().proxy(proxy).build()?;

    // nymtech.net is on the default Nym exit policy. If you change this URL to a
    // destination the exit policy does not allow, the request fails at the exit,
    // which is not a discovery failure.
    println!("Sending request through the mixnet");
    let status = http.get("https://nymtech.net").send().await?.status();
    println!("Got response status: {status}");

    client.disconnect().await;
    Ok(())
}
