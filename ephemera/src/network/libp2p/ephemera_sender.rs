use log::trace;
use tokio::sync::mpsc;

use crate::block::types::message::EphemeraMessage;
use crate::broadcast::RbMsg;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EphemeraEvent {
    EphemeraMessage(Box<EphemeraMessage>),
    ProtocolMessage(Box<RbMsg>),
    StoreInDht { key: Vec<u8>, value: Vec<u8> },
    QueryDht { key: Vec<u8> },
}

pub(crate) struct EphemeraToNetwork;

impl EphemeraToNetwork {
    pub(crate) fn init() -> (EphemeraToNetworkSender, EphemeraToNetworkReceiver) {
        let (net_event_tx, net_event_rcv) = mpsc::channel(1000);

        let receiver = EphemeraToNetworkReceiver::new(net_event_rcv);
        let sender = EphemeraToNetworkSender::new(net_event_tx);

        (sender, receiver)
    }
}

//Receives messages from the network
pub(crate) struct EphemeraToNetworkReceiver {
    pub(crate) net_event_rcv: mpsc::Receiver<EphemeraEvent>,
}

impl EphemeraToNetworkReceiver {
    pub(crate) fn new(net_event_rcv: mpsc::Receiver<EphemeraEvent>) -> Self {
        Self { net_event_rcv }
    }
}

//Sends messages to the network
pub(crate) struct EphemeraToNetworkSender {
    pub(crate) network_event_sender_tx: mpsc::Sender<EphemeraEvent>,
}

impl EphemeraToNetworkSender {
    pub(crate) fn new(network_event_sender_tx: mpsc::Sender<EphemeraEvent>) -> Self {
        Self {
            network_event_sender_tx,
        }
    }

    pub(crate) async fn send_ephemera_event(&mut self, event: EphemeraEvent) -> anyhow::Result<()> {
        trace!("Network event: {:?}", event);
        self.network_event_sender_tx
            .send(event)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
