use nym_sdk::tcp_proxy;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();
    let proxyclient = tcp_proxy::tcp_proxy_client::NymProxyClient::new({ todo!() });
    // serverclient
    // put them both in tasks
    // ping between
}
