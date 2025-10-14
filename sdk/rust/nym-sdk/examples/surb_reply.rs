use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage,
};
use nym_sdk::DebugConfig;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use tracing::warn;
use tracing::instrument;

#[tokio::main]
#[instrument(name = "sdk-example-surb-reply", skip_all)]
async fn main() {
    let _guard = nym_bin_common::opentelemetry::setup_tracing_logger("sdk-example-surb-reply".to_string()).unwrap();

    let tracer = global::tracer("sdk-example-surb-reply");
    let span = tracer.start("client-root-span");
    let cx = Context::current_with_span(span);
    let _guard = cx.clone().attach();

    let trace_id = cx.span().span_context().trace_id();
    warn!("Main TRACE_ID: {:?}", trace_id);

    let context = Context::current();
    println!("Current OTEL context: {:?}", context);

    // Ignore performance requirements for the sake of the example
        let mut debug_config = DebugConfig::default();
        debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
        debug_config
            .traffic
            .disable_main_poisson_packet_distribution = true;

        debug_config.topology.minimum_mixnode_performance = 0;
        debug_config.topology.minimum_gateway_performance = 0;

    // Create a mixnet client which connect to a specific node
    let client_builder  = MixnetClientBuilder::new_ephemeral();
    let mixnet_client = client_builder
        .debug_config(debug_config)
        .request_gateway("FtR9Mb9y9EViYU3at6Qf7MzNHaMw8gofMicwqoscMBMP".to_string())
        .build()
        .unwrap();

    // Now we connect to the mixnet, using keys now stored in the paths provided.
    // let mut client = client.connect_to_mixnet().await.unwrap();

    let mut client = mixnet_client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("\nOur client nym address is: {our_address}");

    // Send a message through the mixnet to ourselves using our nym address
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();
    
    // we're going to parse the sender_tag (AnonymousSenderTag) from the incoming message and use it to 'reply' to ourselves instead of our Nym address.
    // we know there will be a sender_tag since the sdk sends SURBs along with messages by default.
    println!("Waiting for message\n");

    // get the actual message - discard the empty vec sent along with a potential SURB topup request
    let mut message: Vec<ReconstructedMessage> = Vec::new();
    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            continue;
        }
        message = new_message;
        break;
    }

    let mut parsed = String::new();
    if let Some(r) = message.first() {
        parsed = String::from_utf8(r.message.clone()).unwrap();
    }
    // parse sender_tag: we will use this to reply to sender without needing their Nym address
    let return_recipient: AnonymousSenderTag = message[0].sender_tag.unwrap();
    println!(
        "\nReceived the following message: {parsed} \nfrom sender with surb bucket {return_recipient}"
    );

    // reply to self with it: note we use `send_str_reply` instead of `send_str`
    println!("Replying with using SURBs");
    client
        .send_reply(return_recipient, "hi an0n!")
        .await
        .unwrap();

    println!("Waiting for message (once you see it, ctrl-c to exit)\n");
    client
        .on_messages(|msg| println!("\nReceived: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
