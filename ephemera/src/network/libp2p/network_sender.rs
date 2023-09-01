use log::trace;
use std::collections::HashSet;
use tokio::sync::mpsc;

use crate::block::types::message::EphemeraMessage;
use crate::broadcast::RbMsg;
use crate::peer::PeerId;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GroupChangeEvent {
    PeersUpdated(HashSet<PeerId>),
    LocalPeerRemoved(HashSet<PeerId>),
    NotEnoughPeers(HashSet<PeerId>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NetworkEvent {
    EphemeraMessage(Box<EphemeraMessage>),
    BroadcastMessage(Box<RbMsg>),
    GroupUpdate(GroupChangeEvent),
    QueryDhtResponse { key: Vec<u8>, value: Vec<u8> },
}

pub(crate) struct EphemeraNetworkCommunication;

impl EphemeraNetworkCommunication {
    pub(crate) fn init() -> (NetCommunicationSender, NetCommunicationReceiver) {
        let (net_event_tx, net_event_rcv) = mpsc::channel(1000);

        let receiver = NetCommunicationReceiver::new(net_event_rcv);
        let sender = NetCommunicationSender::new(net_event_tx);

        (sender, receiver)
    }
}

//Receives messages from the network
pub(crate) struct NetCommunicationReceiver {
    pub(crate) net_event_rcv: mpsc::Receiver<NetworkEvent>,
}

impl NetCommunicationReceiver {
    pub(crate) fn new(net_event_rcv: mpsc::Receiver<NetworkEvent>) -> Self {
        Self { net_event_rcv }
    }
}

//Sends messages to the network
pub(crate) struct NetCommunicationSender {
    pub(crate) network_event_sender_tx: mpsc::Sender<NetworkEvent>,
}

impl NetCommunicationSender {
    pub(crate) fn new(network_event_sender_tx: mpsc::Sender<NetworkEvent>) -> Self {
        Self {
            network_event_sender_tx,
        }
    }

    pub(crate) async fn send_network_event(&mut self, event: NetworkEvent) -> anyhow::Result<()> {
        trace!("Network event: {:?}", event);
        self.network_event_sender_tx
            .send(event)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
