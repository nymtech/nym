use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage,
};
use nym_topology::{CachedEpochRewardedSet, EntryDetails, HardcodedTopologyProvider, NymTopology, NymTopologyMetadata, RoutingNode, SupportedRoles};
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use time::OffsetDateTime;
use tracing::warn;
use tracing::instrument;

#[tokio::main]
#[instrument(name = "sdk-example-surb-reply", skip_all)]
async fn main() {
    nym_bin_common::opentelemetry::setup_tracing_logger("local-sdk-example-surb-reply".to_string()).unwrap();

    let tracer = global::tracer("local-sdk-example-surb-reply");
    let span = tracer.start("client-root-span");
    let cx = Context::current_with_span(span);
    let _guard = cx.clone().attach();

    let trace_id = cx.span().span_context().trace_id();
    warn!("Main TRACE_ID: {:?}", trace_id);
   
    // Create a mixnet client which connect to a local node
    let topology_metadata = NymTopologyMetadata::new(0, 0, OffsetDateTime::now_utc());
    let mut rewarded_set = CachedEpochRewardedSet::default();
    rewarded_set.entry_gateways.insert(1);
    rewarded_set.layer1.insert(1);
    rewarded_set.layer2.insert(1);
    rewarded_set.layer3.insert(1);

    let nodes = vec![RoutingNode {
        node_id: 1,
        mix_host: "127.0.0.1:1789".parse().unwrap(),
        entry: Some(EntryDetails {
            ip_addresses: vec!["127.0.0.1".parse().unwrap()],
            clients_ws_port: 9000,
            hostname: None,
            clients_wss_port: None,
        }),
            identity_key: "Put Your Identity Key Here"
            .parse()
            .unwrap(),
        sphinx_key: "Put Your Sphinx Key Here"
            .parse()
            .unwrap(),
        supported_roles: SupportedRoles {
            mixnode: true,
            mixnet_entry: true,
            mixnet_exit: true,
        },
    }];

    let topology_provider =
        HardcodedTopologyProvider::new(NymTopology::new(topology_metadata, rewarded_set, nodes));

    let mut client = MixnetClientBuilder::new_ephemeral()
        .custom_topology_provider(Box::new(topology_provider))
        .build().unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

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
