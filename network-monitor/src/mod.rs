use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use log::error;
use notifications::Notifier;

use packet_sender::PacketSender;
use tokio::runtime::Runtime;

mod chunker;
pub(crate) mod good_topology;
pub(crate) mod notifications;
pub(crate) mod packet_sender;

type MixnetReceiver = UnboundedReceiver<Vec<Vec<u8>>>;
pub(crate) type MixnetSender = UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type AckSender = UnboundedSender<Vec<Vec<u8>>>;

pub struct Monitor {}

impl Monitor {
    pub fn new() -> Monitor {
        Monitor {}
    }

    pub(crate) fn run(&mut self, mut notifier: Notifier, mut packet_sender: PacketSender) {
        let mut runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            tokio::spawn(async move {
                notifier.run().await;
            });

            // tokio::spawn(async move {
            //     let mut interval = time::interval(time::Duration::from_secs(2));
            //     loop {
            packet_sender.sanity_check().await;
            packet_sender.send_packets_to_all_nodes().await;
            //         interval.tick().await;
            //     }
            // });

            self.wait_for_interrupt().await
        });
    }

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!("Received SIGINT - the network monitor will terminate now");
    }
}
