//! SOCKS5 proxy client that routes an HTTPS request through the mixnet.
//!
//! Picks a network requester (the mixnet exit), starts a local SOCKS5 listener,
//! points an HTTP client at it, and makes a real request end to end.
//!
//! Run with: cargo run --example socks5

use nym_sdk::mixnet::{NetworkRequesterSelector, Socks5MixnetClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    nym_bin_common::logging::setup_tracing_logger();

    // How to choose the requester. `any()` auto-discovers one from the directory,
    // weighted by performance, and is the most robust default. The alternatives:
    //   NetworkRequesterSelector::in_countries(["CH", "DE"])? -> discovery pinned to those countries
    //   NetworkRequesterSelector::exact(Entztfv6Uaz2hpYHQJ6JKoaCTpDL5dja18SuQWVJAmmx.Cvhn9rBJw5Ay9wgHcbgCnVg89MPSV5s2muPV2YF1BXYu@Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf)? -> a specific known requester
    // If you already have the address, Socks5MixnetClient::connect_new(Entztfv6Uaz2hpYHQJ6JKoaCTpDL5dja18SuQWVJAmmx.Cvhn9rBJw5Ay9wgHcbgCnVg89MPSV5s2muPV2YF1BXYu@Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf) is the one-line shorthand for the `exact` case.
    let requester = NetworkRequesterSelector::any();

    // Passing `None` binds the SOCKS5 listener to the default 127.0.0.1:1080. Pass Some(addr)
    // to move it, for example when that port is taken or to run more than one client.
    println!("Selecting a network requester and connecting");
    let client =
        Socks5MixnetClient::connect_with(requester, Some("127.0.0.1:1081".parse()?)).await?;
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
