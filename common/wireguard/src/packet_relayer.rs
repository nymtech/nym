use std::{collections::HashMap, sync::Arc};

use tap::TapFallible;
use tokio::sync::mpsc::{self};

use crate::{
    active_peers::PeerEventSender,
    event::Event,
    tun_task_channel::{TunTaskResponseRx, TunTaskTx},
};

#[derive(Clone)]
pub struct PacketRelaySender(pub(crate) mpsc::Sender<(u64, Vec<u8>)>);
pub(crate) struct PacketRelayReceiver(pub(crate) mpsc::Receiver<(u64, Vec<u8>)>);

pub(crate) fn packet_relay_channel() -> (PacketRelaySender, PacketRelayReceiver) {
    let (tx, rx) = mpsc::channel(16);
    (PacketRelaySender(tx), PacketRelayReceiver(rx))
}

// The tunnels send packets to the packet relayer, which then relays it to the tun device. And
// conversely, it's where the tun device send responses to, which are relayed back to the correct
// tunnel.
pub(crate) struct PacketRelayer {
    // Receive packets from the various tunnels
    packet_rx: PacketRelayReceiver,

    // After receive from tunnels, send to the tun device
    tun_task_tx: TunTaskTx,

    // Receive responses from the tun device
    tun_task_response_rx: TunTaskResponseRx,

    // After receiving from the tun device, relay back to the correct tunnel
    peers_by_tag: Arc<tokio::sync::Mutex<HashMap<u64, PeerEventSender>>>,
}

impl PacketRelayer {
    pub(crate) fn new(
        tun_task_tx: TunTaskTx,
        tun_task_response_rx: TunTaskResponseRx,
        peers_by_tag: Arc<tokio::sync::Mutex<HashMap<u64, PeerEventSender>>>,
    ) -> (Self, PacketRelaySender) {
        let (packet_tx, packet_rx) = packet_relay_channel();
        (
            Self {
                packet_rx,
                tun_task_tx,
                tun_task_response_rx,
                peers_by_tag,
            },
            packet_tx,
        )
    }

    pub(crate) async fn run(mut self) {
        loop {
            tokio::select! {
                Some((tag, packet)) = self.packet_rx.0.recv() => {
                    log::info!("Sent packet to tun device with tag: {tag}");
                    self.tun_task_tx.send((tag, packet)).await.tap_err(|e| log::error!("{e}")).ok();
                },
                Some((tag, packet)) = self.tun_task_response_rx.recv() => {
                    log::info!("Received response from tun device with tag: {tag}");
                    if let Some(tx) = self.peers_by_tag.lock().await.get(&tag) {
                        tx.send(Event::Ip(packet.into())).await.tap_err(|e| log::error!("{e}")).ok();
                    }
                }
            }
        }
    }

    pub(crate) fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}
