use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage,
};
use nym_sdk::DebugConfig;
#[cfg(feature = "otel")]
use opentelemetry::trace::TraceContextExt;
#[cfg(feature = "otel")]
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing::warn;
use tracing::instrument;
#[cfg(feature = "otel")]
use tracing::Instrument;

#[tokio::main]
#[instrument(name = "sdk-example-surb-reply", skip_all)]
async fn main() {
    // Setup OpenTelemetry tracing
    #[cfg(feature = "otel")]
    {
        let _guard = nym_bin_common::opentelemetry::setup_tracing_logger("sdk-example-surb-reply".to_string()).unwrap();
        let main_span = tracing::info_span!("startup", service = "sdk-example-surb-reply");
        async {
            let tracing_context = tracing::Span::current();
            tracing::info!("Current tracing context: {:?}", tracing_context);
            let cx = tracing::Span::current().context();
            let sc = cx.span();
            let spcx = sc.span_context();
            tracing::debug!("Current OTEL context: {:?}, trace_id: {:?}", cx, spcx.trace_id());

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
        }.instrument(main_span).await;
    }

    // due to the way instrumentation works in async contexts, the totality of the async code we want to trace needs to be under the same block
    // This is ugly and unfortunate but cannot think of another way around it right now.
    #[cfg(not(feature = "otel"))]
    {
        nym_bin_common::logging::setup_no_otel_logger().expect("failed to initialize logging");
        
        let mut debug_config = DebugConfig::default();
        debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
        debug_config
            .traffic
            .disable_main_poisson_packet_distribution = true;

        debug_config.topology.minimum_mixnode_performance = 0;
        debug_config.topology.minimum_gateway_performance = 0;

        let client_builder  = MixnetClientBuilder::new_ephemeral();
        let mixnet_client = client_builder
            .debug_config(debug_config)
            .request_gateway("FtR9Mb9y9EViYU3at6Qf7MzNHaMw8gofMicwqoscMBMP".to_string())
            .build()
            .unwrap();

        let mut client = mixnet_client.connect_to_mixnet().await.unwrap();

        let our_address = client.nym_address();
        println!("\nOur client nym address is: {our_address}");

        client
            .send_plain_message(*our_address, "hello there")
            .await
            .unwrap();
        
        println!("Waiting for message\n");

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
        let return_recipient: AnonymousSenderTag = message[0].sender_tag.unwrap();
        println!(
            "\nReceived the following message: {parsed} \nfrom sender with surb bucket {return_recipient}"
        );

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
}


// #[tokio::main]
// #[instrument(name = "sdk-example-surb-reply", skip_all)]
// async fn main() {
//     // Setup OpenTelemetry tracing
//     #[cfg(feature = "otel")]
//     let _guard = nym_bin_common::opentelemetry::setup_tracing_logger("sdk-example-surb-reply".to_string()).unwrap();
//     #[cfg(feature = "otel")]
//     let main_span = tracing::info_span!("startup", service = "sdk-example-surb-reply");
//     #[cfg(feature = "otel")]
//     let _main_span_guard = main_span.enter();
//     #[cfg(feature = "otel")]
//     let tracing_context = tracing::Span::current();
//     #[cfg(feature = "otel")]
//     tracing::info!("Current tracing context: {:?}", tracing_context);
//     #[cfg(feature = "otel")]
//     let otel_context = Context::current();
//     #[cfg(feature = "otel")]
//     tracing::info!("Current OTEL context: {:?}", otel_context);

//     #[cfg(not(feature = "otel"))]
//     nym_bin_common::logging::setup_no_otel_logger().expect("failed to initialize logging");

//     // Ignore performance requirements for the sake of the example
//         let mut debug_config = DebugConfig::default();
//         debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
//         debug_config
//             .traffic
//             .disable_main_poisson_packet_distribution = true;

//         debug_config.topology.minimum_mixnode_performance = 0;
//         debug_config.topology.minimum_gateway_performance = 0;

//     // Create a mixnet client which connect to a specific node
//     let client_builder  = MixnetClientBuilder::new_ephemeral();
//     let mixnet_client = client_builder
//         .debug_config(debug_config)
//         .request_gateway("FtR9Mb9y9EViYU3at6Qf7MzNHaMw8gofMicwqoscMBMP".to_string())
//         .build()
//         .unwrap();

//     // Now we connect to the mixnet, using keys now stored in the paths provided.
//     // let mut client = client.connect_to_mixnet().await.unwrap();

//     let mut client = mixnet_client.connect_to_mixnet().await.unwrap();

//     // Be able to get our client address
//     let our_address = client.nym_address();
//     println!("\nOur client nym address is: {our_address}");

//     // Send a message through the mixnet to ourselves using our nym address
//     client
//         .send_plain_message(*our_address, "hello there")
//         .await
//         .unwrap();
    
//     // we're going to parse the sender_tag (AnonymousSenderTag) from the incoming message and use it to 'reply' to ourselves instead of our Nym address.
//     // we know there will be a sender_tag since the sdk sends SURBs along with messages by default.
//     println!("Waiting for message\n");

//     // get the actual message - discard the empty vec sent along with a potential SURB topup request
//     let mut message: Vec<ReconstructedMessage> = Vec::new();
//     while let Some(new_message) = client.wait_for_messages().await {
//         if new_message.is_empty() {
//             continue;
//         }
//         message = new_message;
//         break;
//     }

//     let mut parsed = String::new();
//     if let Some(r) = message.first() {
//         parsed = String::from_utf8(r.message.clone()).unwrap();
//     }
//     // parse sender_tag: we will use this to reply to sender without needing their Nym address
//     let return_recipient: AnonymousSenderTag = message[0].sender_tag.unwrap();
//     println!(
//         "\nReceived the following message: {parsed} \nfrom sender with surb bucket {return_recipient}"
//     );

//     // reply to self with it: note we use `send_str_reply` instead of `send_str`
//     println!("Replying with using SURBs");
//     client
//         .send_reply(return_recipient, "hi an0n!")
//         .await
//         .unwrap();

//     println!("Waiting for message (once you see it, ctrl-c to exit)\n");
//     client
//         .on_messages(|msg| println!("\nReceived: {}", String::from_utf8_lossy(&msg.message)))
//         .await;
// }
