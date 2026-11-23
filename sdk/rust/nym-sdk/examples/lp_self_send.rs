//! Send a message to yourself over the Lewes Protocol (LP).
//!
//! The client never opens a websocket: it handshakes and registers with its gateway over LP on
//! TCP, then sends sphinx packets over UDP. The message goes gateway → mix nodes → gateway and
//! comes back to the same client.
//!
//! ```sh
//! # sandbox, a random gateway from the LP topology
//! cargo run --example lp_self_send -- sandbox
//!
//! # mainnet, a gateway of your choice (its identity key)
//! cargo run --example lp_self_send -- mainnet <gateway-identity>
//! ```
//!

use futures::StreamExt;
use nym_sdk::mixnet::{self, IncludedSurbs, MixnetMessageSender};
use std::time::Duration;

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    let mut args = std::env::args().skip(1);
    let network = args.next().unwrap_or_else(|| "sandbox".to_string());
    let gateway = args.next();

    let network_details = match network.as_str() {
        "sandbox" => mixnet::NymNetworkDetails::new_sandbox(),
        "mainnet" => mixnet::NymNetworkDetails::new_mainnet(),
        other => anyhow::bail!("unknown network '{other}', expected 'sandbox' or 'mainnet'"),
    };

    let mut builder = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(network_details)
        .with_lp(true);
    if let Some(gateway) = gateway {
        builder = builder.request_gateway(gateway);
    }

    let mut client = builder.build()?.connect_to_mixnet().await?;
    let our_address = *client.nym_address();
    println!("Connected over LP on {network}, our address is {our_address}");

    // LP carries no reply SURBs yet, so the message goes without any: the recipient (us) gets the
    // payload and nothing about who sent it.
    client
        .send_message(
            our_address,
            "hello over LP",
            IncludedSurbs::ExposeSelfAddress,
        )
        .await?;
    println!(
        "Sent, waiting up to {}s for it to come back",
        RECEIVE_TIMEOUT.as_secs()
    );

    match tokio::time::timeout(RECEIVE_TIMEOUT, client.next()).await {
        Ok(Some(received)) => {
            println!("Received: {}", String::from_utf8_lossy(&received.message))
        }
        Ok(None) => eprintln!("The client shut down before anything arrived"),
        Err(_) => eprintln!(
            "Nothing arrived within {}s. Check the logs for a missing LP session or route",
            RECEIVE_TIMEOUT.as_secs()
        ),
    }

    client.disconnect().await;
    Ok(())
}
