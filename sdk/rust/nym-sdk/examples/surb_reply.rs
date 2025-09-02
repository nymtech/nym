use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage,
    // StoragePaths,
};
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, Context};
// use opentelemetry::trace::Tracer;
// use opentelemetry::Context;
// use opentelemetry::global;
use tracing::warn;
use tracing::{instrument, info_span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use std::path::PathBuf;
use tempfile::TempDir;
use nym_sdk::mixnet::StoragePaths;

#[tokio::main]
#[instrument(name = "sdk-example-surb-reply", skip_all)]
async fn main() {
    nym_bin_common::opentelemetry::setup_tracing_logger("sdk-example-surb-reply".to_string()).unwrap();

    // let main_span = info_span!("surb_example_session");
    // let _main_span_enter = main_span.enter();

    // let current_span = tracing::Span::current();
    // let otel_context = current_span.context();
    // let binding = otel_context.span();
    // let span_context = binding.span_context();
    // let trace_id = span_context.trace_id();
    
    // warn!("Starting the SURB reply example - trace id: {}", trace_id);
    // warn!("Otel context: {:?}", otel_context);
    // warn!("trace id: {}", trace_id);
    let tracer = global::tracer("sdk-example-surb-reply");
    let span = tracer.start("test_span");
    let cx = Context::current_with_span(span);
    let _guard = cx.clone().attach();

    let trace_id = cx.span().span_context().trace_id();
    warn!("Main TRACE_ID: {:?}", trace_id);

    let span = info_span!(
        "surb_reply_example_session",
        trace_id = %trace_id.to_string()
    );
    let _enter = span.enter();

    let otel_context = opentelemetry::Context::current();
    warn!("OTEL CONTEXT: {:?}", otel_context);
    let span = otel_context.span();
    let context = span.span_context();
    let trace_id = context.trace_id();
    warn!("TRACE_ID: {:?}", trace_id);
   
    // Specify some config options
    let config_dir: PathBuf = TempDir::new().unwrap().path().to_path_buf();
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();

    // Create the client with a storage backend, and enable it by giving it some paths. If keys
    // exists at these paths, they will be loaded, otherwise they will be generated.
    // let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
    //     .await
    //     .unwrap()
    //     .build()
    //     .unwrap();

    // Now we connect to the mixnet, using keys now stored in the paths provided.
    // let mut client = client.connect_to_mixnet().await.unwrap();

    // Create a mixnet client which connect to a specific node

    let client_builder  = MixnetClientBuilder::new_ephemeral();
    let mixnet_client = client_builder
        .request_gateway("BAF2aYpzcK9KbSS3Y7EdLisxiogkTr88FXkdL8EDNigH".to_string())
        .build()
        .unwrap();

    let mut client = mixnet_client.connect_to_mixnet().await.unwrap();
    // Now we connect to the mixnet, using keys now stored in the paths provided.
    // let mut client = client.connect_to_mixnet().await.unwrap();

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
