use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc::{self};

use crate::{event::Event, tun_task_channel::TunTaskTx};

// The tunnels send packets to the packet relayer, which then relays it to the tun device. And
// conversely, it's where the tun device send responses to, which are relayed back to the correct
// tunnel.
pub(crate) struct PacketRelayer {
    // Receive packets from the various tunnels
    packet_rx: mpsc::Receiver<(u64, Vec<u8>)>,

    // After receive from tunnels, send to the tun device
    tun_task_tx: TunTaskTx,

    // Receive responses from the tun device
    // tun_task_rx: TunTaskRx,

    // After receiving from the tun device, relay back to the correct tunnel
    peers_by_tag: Arc<std::sync::Mutex<HashMap<u64, mpsc::UnboundedSender<Event>>>>,
}

impl PacketRelayer {
    pub(crate) fn new(
        tun_task_tx: TunTaskTx,
        // tun_task_rx: TunTaskRx,
        peers_by_tag: Arc<std::sync::Mutex<HashMap<u64, mpsc::UnboundedSender<Event>>>>,
    ) -> (Self, mpsc::Sender<(u64, Vec<u8>)>) {
        let (packet_tx, packet_rx) = mpsc::channel(16);
        (
            Self {
                packet_rx,
                tun_task_tx,
                // tun_task_rx,
                peers_by_tag,
            },
            packet_tx,
        )
    }

    pub(crate) async fn run(mut self) {
        loop {
            tokio::select! {
                Some((tag, packet)) = self.packet_rx.recv() => {
                    self.tun_task_tx.send((tag, packet)).unwrap();
                }
            }
        }
    }

    pub(crate) fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}
