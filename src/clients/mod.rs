use sphinx::route::{DestinationAddressBytes, NodeAddressBytes, Destination};
use tokio::runtime::Runtime;
use futures::channel::mpsc;
use std::time::Duration;
use crate::utils;
use futures::{future, StreamExt, SinkExt};
use crate::clients::mix::MixClient;
use sphinx::SphinxPacket;

pub mod directory;
pub mod mix;
pub mod provider;
pub mod validator;


// TODO: put that in config once it exists
const LOOP_COVER_AVERAGE_DELAY: f64 = 10.0;
// assume seconds
const MESSAGE_SENDING_AVERAGE_DELAY: f64 = 10.0;
// assume seconds;
const FETCH_MESSAGES_DELAY: f64 = 10.0; // assume seconds;


// provider-poller sends polls service provider; receives messages
// provider-poller sends (TX) to ReceivedBufferController (RX)
// ReceivedBufferController sends (TX) to ... ??Client??
// outQueueController sends (TX) to TrafficStreamController (RX)
// TrafficStreamController sends messages to mixnet
// ... ??Client?? sends (TX) to outQueueController (RX)
// Loop cover traffic stream just sends messages to mixnet without any channel communication

struct MixMessage(NodeAddressBytes, SphinxPacket);

struct MixTrafficController;



impl MixTrafficController {
    // this was way more difficult to implement than what this code may suggest...
    async fn run(mut rx: mpsc::UnboundedReceiver<MixMessage>) {
        let mix_client = MixClient::new();
        while let Some(mix_message) = rx.next().await {
            println!("got a mix_message for {:?}", mix_message.0);

            println!("here i will be sending sphinx packet to a mixnode ({:?}!", mix_message.0);
            // here NodeAddressBytes would be transformed into a SocketAddr with SOME library call...
            let node_net_address = "127.0.0.1:8080";
            let send_res = mix_client.send(mix_message.1, node_net_address.parse().unwrap()).await;
            match send_res {
                Ok(_) => println!("We successfully sent the message!"),
                Err(e) => eprintln!("We failed to send the message :( - {:?}", e),
            };
        }
    }
}

pub struct NymClient {
    // to be replaced by something else I guess
    address: DestinationAddressBytes,
    pub input_tx: mpsc::UnboundedSender<Vec<u8>>,
    // to be used by "send" function or socket, etc
    input_rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

type TripleFutureResult = (Result<(), Box<dyn std::error::Error>>, Result<(), Box<dyn std::error::Error>>, Result<(), Box<dyn std::error::Error>>);

impl NymClient {
    pub fn new(address: DestinationAddressBytes) -> Self {
        let (input_tx, input_rx) = mpsc::unbounded::<Vec<u8>>();

        NymClient {
            address,
            input_tx,
            input_rx,
        }
    }

    async fn start_loop_cover_traffic_stream(mut tx: mpsc::UnboundedSender<MixMessage>, our_info: Destination) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let delay = utils::poisson::sample(LOOP_COVER_AVERAGE_DELAY);
            let delay_duration = Duration::from_secs_f64(delay);
            println!("waiting for {:?}", delay_duration);
            tokio::time::delay_for(delay_duration).await;
            let cover_message = utils::sphinx::loop_cover_message(our_info.address, our_info.identifier);
            println!("waited {:?} - time to send cover message!", delay_duration);
            tx.send(MixMessage(cover_message.0, cover_message.1)).await?;
        }
    }

    async fn control_out_queue(mut mix_tx: mpsc::UnboundedSender<MixMessage>, mut input_rx: mpsc::UnboundedReceiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            println!("here I will be sending real traffic (or loop cover if nothing is available)");
            let delay_duration = Duration::from_secs_f64(10.0);
            tokio::time::delay_for(delay_duration).await;
        }
    }


    async fn start_provider_polling() -> Result<(), Box<dyn std::error::Error>> {
        loop {
            println!("here I will be polling provider for messages");
            let delay_duration = Duration::from_secs_f64(10.0);
            tokio::time::delay_for(delay_duration).await;
        }
    }


    pub fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        println!("starting nym client");

        let (mix_tx, mix_rx) = mpsc::unbounded();
        let our_info = Destination::new(self.address, Default::default());

        let mut rt = Runtime::new()?;

        rt.spawn(MixTrafficController::run(mix_rx));

        rt.block_on(async {
            let future_results = futures::future::join3(
                NymClient::start_loop_cover_traffic_stream(mix_tx.clone(), our_info),
                NymClient::control_out_queue(mix_tx, self.input_rx),
                NymClient::start_provider_polling()).await;
            assert!(future_results.0.is_ok() && future_results.1.is_ok() && future_results.2.is_ok());
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on traffic senders
        eprintln!("The client went kaput...");
        Ok(())
    }
}