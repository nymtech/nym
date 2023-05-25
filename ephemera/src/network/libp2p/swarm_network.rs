use std::collections::HashSet;
use std::future::Future;
use std::str::FromStr;

use futures::StreamExt;
use libp2p::kad::{GetClosestPeersResult, GetRecordResult};
use libp2p::swarm::{NetworkBehaviour, SwarmBuilder};
use libp2p::{
    gossipsub, gossipsub::IdentTopic as Topic, kad, request_response, swarm::SwarmEvent, Multiaddr,
    Swarm,
};
use log::{debug, error, info, trace};

use crate::membership::PeerInfo;
use crate::{
    block::types::message::EphemeraMessage,
    broadcast::RbMsg,
    codec::Encode,
    core::builder::NodeInfo,
    network::libp2p::behaviours,
    network::libp2p::{
        behaviours::{
            create_behaviour, create_transport, request_response::RbMsgResponse,
            GroupBehaviourEvent, GroupNetworkBehaviour,
        },
        ephemera_sender::{
            EphemeraEvent, EphemeraToNetwork, EphemeraToNetworkReceiver, EphemeraToNetworkSender,
        },
        network_sender::{
            EphemeraNetworkCommunication, GroupChangeEvent,
            GroupChangeEvent::{LocalPeerRemoved, NotEnoughPeers},
            NetCommunicationReceiver, NetCommunicationSender, NetworkEvent,
        },
    },
};

pub(crate) type InitSwarm<P> = (
    SwarmNetwork<P>,
    NetCommunicationReceiver,
    EphemeraToNetworkSender,
);

pub struct SwarmNetwork<P>
where
    P: Future<Output = crate::membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
{
    node_info: NodeInfo,
    swarm: Swarm<GroupNetworkBehaviour<P>>,
    from_ephemera_rcv: EphemeraToNetworkReceiver,
    to_ephemera_tx: NetCommunicationSender,
    ephemera_msg_topic: Topic,
}

impl<P> SwarmNetwork<P>
where
    P: Future<Output = crate::membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
{
    pub(crate) fn new(node_info: NodeInfo, members_provider: P) -> anyhow::Result<InitSwarm<P>>
    where
        P: Future<Output = crate::membership::Result<Vec<PeerInfo>>> + Send + 'static,
    {
        let (from_ephemera_tx, from_ephemera_rcv) = EphemeraToNetwork::init();
        let (to_ephemera_tx, to_ephemera_rcv) = EphemeraNetworkCommunication::init();

        let libp2p_configuration = node_info.initial_config.libp2p.clone();

        let local_key = node_info.keypair.clone();
        let peer_id = node_info.peer_id;
        let ephemera_msg_topic = Topic::new(&libp2p_configuration.ephemera_msg_topic_name);

        let transport = create_transport(&local_key)?;

        let members_provider_delay =
            std::time::Duration::from_secs(libp2p_configuration.members_provider_delay_sec);
        let behaviour = create_behaviour(
            &local_key,
            &ephemera_msg_topic,
            members_provider,
            members_provider_delay,
            libp2p_configuration.membership_kind.into(),
        );

        let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id.into()).build();

        let network = SwarmNetwork {
            node_info,
            swarm,
            from_ephemera_rcv,
            to_ephemera_tx,
            ephemera_msg_topic,
        };

        Ok((network, to_ephemera_rcv, from_ephemera_tx))
    }

    pub(crate) fn listen(&mut self) -> anyhow::Result<()> {
        let address =
            Multiaddr::from_str(&self.node_info.protocol_address()).expect("Invalid multi-address");
        self.swarm.listen_on(address.clone())?;

        info!("Listening on {address:?}");
        Ok(())
    }

    pub(crate) async fn start(mut self) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                swarm_event = self.swarm.next() => {
                    match swarm_event{
                        Some(event) => {
                            if let Err(err) = self.handle_incoming_messages(event).await{
                                error!("Error handling swarm event: {:?}", err);
                            }
                        }
                        None => {
                            anyhow::bail!("Swarm event channel closed");
                        }
                    }
                },
                Some(event) = self.from_ephemera_rcv.net_event_rcv.recv() => {
                    self.process_ephemera_events(event);
                }
            }
        }
    }

    fn process_ephemera_events(&mut self, event: EphemeraEvent) {
        match event {
            EphemeraEvent::EphemeraMessage(em) => {
                self.send_ephemera_message(em.as_ref());
            }
            EphemeraEvent::ProtocolMessage(pm) => {
                self.send_broadcast_message(pm.as_ref());
            }
            EphemeraEvent::StoreInDht { key, value } => {
                let record = kad::Record::new(key, value);
                let quorum = kad::Quorum::One;
                match self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, quorum)
                {
                    Ok(ok) => {
                        trace!("StoreDht: {:?}", ok);
                    }
                    Err(err) => {
                        error!("StoreDht: {:?}", err);
                    }
                }
            }
            EphemeraEvent::QueryDht { key } => {
                let kad_key = kad::record::Key::new::<Vec<u8>>(key.as_ref());
                let query_id = self.swarm.behaviour_mut().kademlia.get_record(kad_key);
                trace!("QueryDht: {:?}", query_id);
            }
        }
    }

    async fn handle_incoming_messages<E>(
        &mut self,
        swarm_event: SwarmEvent<GroupBehaviourEvent, E>,
    ) -> anyhow::Result<()> {
        if let SwarmEvent::Behaviour(b) = swarm_event {
            if let Err(err) = self.process_group_behaviour_event(b).await {
                error!("Error handling behaviour event: {:?}", err);
            }
        } else {
            Self::process_other_swarm_events(swarm_event);
        }
        Ok(())
    }

    async fn process_group_behaviour_event(
        &mut self,
        event: GroupBehaviourEvent,
    ) -> anyhow::Result<()> {
        match event {
            GroupBehaviourEvent::Gossipsub(gs) => {
                if let Err(err) = self.process_gossipsub_event(gs).await {
                    error!("Error processing gossipsub event: {:?}", err);
                }
            }
            GroupBehaviourEvent::RequestResponse(request_response) => {
                if let Err(err) = self.process_request_response(request_response).await {
                    error!("Error processing request response: {:?}", err);
                }
            }

            GroupBehaviourEvent::Membership(event) => {
                if let Err(err) = self.process_members_provider_event(event).await {
                    error!("Error processing rendezvous event: {:?}", err);
                }
            }
            GroupBehaviourEvent::Kademlia(ev) => {
                if let Err(err) = self.process_kad_event(ev).await {
                    error!("Error processing kademlia event: {:?}", err);
                }
            } // GroupBehaviourEvent::Ping(_) => {}
        }
        Ok(())
    }

    async fn process_gossipsub_event(&mut self, event: gossipsub::Event) -> anyhow::Result<()> {
        match event {
            gossipsub::Event::Message {
                propagation_source: _,
                message_id: _,
                message,
            } => {
                let msg: EphemeraMessage = serde_json::from_slice(&message.data[..])?;
                self.to_ephemera_tx
                    .send_network_event(NetworkEvent::EphemeraMessage(msg.into()))
                    .await?;
            }

            gossipsub::Event::Subscribed { peer_id, topic } => {
                trace!("Peer {peer_id:?} subscribed to topic {topic:?}");
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                trace!("Peer {peer_id:?} unsubscribed from topic {topic:?}");
            }
            gossipsub::Event::GossipsubNotSupported { peer_id } => {
                trace!("Peer {peer_id:?} does not support gossipsub");
            }
        }
        Ok(())
    }

    async fn process_request_response(
        &mut self,
        event: request_response::Event<RbMsg, RbMsgResponse>,
    ) -> anyhow::Result<()> {
        match event {
            request_response::Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request_id: _,
                    request,
                    channel,
                } => {
                    let rb_id = request.id.clone();
                    trace!("Received request {:?}", request);
                    self.to_ephemera_tx
                        .send_network_event(NetworkEvent::BroadcastMessage(request.into()))
                        .await?;
                    if let Err(err) = self
                        .swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, RbMsgResponse::new(rb_id))
                    {
                        error!("Error sending response: {:?}", err);
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    trace!("Received response {response:?} from peer: {peer:?}, request_id: {request_id:?}",);
                }
            },
            request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                error!("Outbound failure: {error:?}, peer:{peer:?}, request_id:{request_id:?}",);
            }
            request_response::Event::InboundFailure {
                peer,
                request_id,
                error,
            } => {
                error!("Inbound failure: {error:?}, peer:{peer:?}, request_id:{request_id:?}",);
            }
            request_response::Event::ResponseSent { peer, request_id } => {
                trace!("Response sent to peer: {peer:?}, {request_id:?}",);
            }
        }
        Ok(())
    }

    async fn process_members_provider_event(
        &mut self,
        event: behaviours::membership::behaviour::Event,
    ) -> anyhow::Result<()> {
        match event {
            behaviours::membership::behaviour::Event::PeersUpdated(peers_ids) => {
                info!("Peers updated: {:?}", peers_ids);

                let local_peer_id = *self.swarm.local_peer_id();
                for peer_id in peers_ids {
                    if peer_id == local_peer_id {
                        continue;
                    }
                    //FIXME: deprecated
                    #[allow(deprecated)]
                    let address = self
                        .swarm
                        .behaviour_mut()
                        .members_provider
                        .addresses_of_peer(&peer_id);
                    if let Some(address) = address.first() {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, address.clone());
                    }
                }

                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .get_closest_peers(libp2p::PeerId::random());
                debug!("Neighbours: {:?}", query_id);
            }
            behaviours::membership::behaviour::Event::PeerUpdatePending => {
                info!("Peer update pending");
            }
            behaviours::membership::behaviour::Event::LocalRemoved(peers_ids) => {
                //TODO: should pause all network block and message activities...?
                let peers_ids = peers_ids.into_iter().map(Into::into).collect();
                let update = NetworkEvent::GroupUpdate(LocalPeerRemoved(peers_ids));
                self.to_ephemera_tx.send_network_event(update).await?;
            }
            behaviours::membership::behaviour::Event::NotEnoughPeers(peers_ids) => {
                //TODO: should pause all network block and message activities...?
                let peers_ids = peers_ids.into_iter().map(Into::into).collect();
                let update = NetworkEvent::GroupUpdate(NotEnoughPeers(peers_ids));
                self.to_ephemera_tx.send_network_event(update).await?;
            }
        }
        Ok(())
    }

    async fn process_kad_event(&mut self, event: kad::KademliaEvent) -> anyhow::Result<()> {
        match event {
            kad::KademliaEvent::OutboundQueryProgressed {
                id,
                result,
                stats,
                step,
            } => {
                trace!(
                    "Outbound query progressed: id:{:?}, result:{:?}, stats:{:?}, step:{:?}",
                    id,
                    result,
                    stats,
                    step
                );
                match result {
                    kad::QueryResult::GetClosestPeers(gcp) => {
                        self.process_closest_peers(gcp).await?;
                    }
                    kad::QueryResult::GetRecord(get_res) => {
                        self.process_get_record(get_res).await?;
                    }
                    kad::QueryResult::Bootstrap(bt) => {
                        trace!("Bootstrap: {:?}", bt);
                    }
                    kad::QueryResult::GetProviders(gp) => {
                        trace!("GetProviders: {:?}", gp);
                    }
                    kad::QueryResult::StartProviding(sp) => {
                        trace!("StartProviding: {:?}", sp);
                    }
                    kad::QueryResult::RepublishProvider(rp) => {
                        trace!("RepublishProvider: {:?}", rp);
                    }
                    kad::QueryResult::PutRecord(pr) => {
                        trace!("PutRecord: {:?}", pr);
                    }
                    kad::QueryResult::RepublishRecord(rr) => {
                        trace!("RepublishRecord: {:?}", rr);
                    }
                }
            }

            kad::KademliaEvent::InboundRequest { request } => {
                trace!("Inbound request: {:?}", request);
            }

            kad::KademliaEvent::RoutingUpdated {
                peer: peer_id,
                is_new_peer: _,
                addresses,
                bucket_range: _,
                old_peer: _,
            } => {
                trace!("Routing updated: peer:{peer_id}, addresses:{addresses:?}",);
            }
            kad::KademliaEvent::UnroutablePeer { peer } => {
                trace!("Unroutable peer: {:?}", peer);
            }
            kad::KademliaEvent::RoutablePeer { peer, address } => {
                trace!("Routable peer: {:?}, address: {:?}", peer, address);
            }
            kad::KademliaEvent::PendingRoutablePeer { peer, address } => {
                trace!("Pending routable peer: {:?}, address: {:?}", peer, address);
            }
        }
        Ok(())
    }

    async fn process_get_record(&mut self, get_res: GetRecordResult) -> anyhow::Result<()> {
        trace!("GetRecord: {:?}", get_res);
        match get_res {
            Ok(ok) => match ok {
                kad::GetRecordOk::FoundRecord(fr) => {
                    let record = fr.record;
                    let event = NetworkEvent::QueryDhtResponse {
                        key: record.key.to_vec(),
                        value: record.value,
                    };
                    self.to_ephemera_tx.send_network_event(event).await?;
                }
                kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. } => {
                    trace!("FinishedWithNoAdditionalRecord");
                }
            },
            Err(err) => {
                trace!("Not getting record: {:?}", err);
            }
        }
        Ok(())
    }

    async fn process_closest_peers(&mut self, gcp: GetClosestPeersResult) -> anyhow::Result<()> {
        trace!("GetClosestPeers: {:?}", gcp);
        //TODO: we need also to make sure that we have enough peers
        // (Repeat if not enough, may need to wait network to stabilize)
        match gcp {
            Ok(cp) => {
                if cp.peers.is_empty() {
                    log::warn!("No peers found");
                    return Ok(());
                }

                let gossipsub = &mut self.swarm.behaviour_mut().gossipsub;
                for peer_id in cp.peers {
                    gossipsub.add_explicit_peer(&peer_id);
                }

                let active_peers = self
                    .swarm
                    .behaviour_mut()
                    .members_provider
                    .active_peer_ids_with_local();
                let active_peers = active_peers
                    .into_iter()
                    .map(Into::into)
                    .collect::<HashSet<_>>();
                let group_update =
                    NetworkEvent::GroupUpdate(GroupChangeEvent::PeersUpdated(active_peers));
                self.to_ephemera_tx.send_network_event(group_update).await?;
            }
            Err(err) => {
                error!("Error getting closest peers: {:?}", err);
            }
        }
        Ok(())
    }

    fn send_broadcast_message(&mut self, msg: &RbMsg) {
        trace!("Sending broadcast message: {:?}", msg);
        let local_peer_id = *self.swarm.local_peer_id();
        let behaviours = self.swarm.behaviour_mut();
        for peer in behaviours.members_provider.active_peer_ids() {
            trace!("Sending broadcast message: {:?} to peer: {peer:?}", msg.id,);
            if *peer == local_peer_id {
                continue;
            }
            behaviours.request_response.send_request(peer, msg.clone());
        }
    }

    fn send_ephemera_message(&mut self, msg: &EphemeraMessage) {
        trace!("Sending Ephemera message: {:?}", msg);
        match msg.encode() {
            Ok(vec) => {
                let topic = self.ephemera_msg_topic.clone();
                if let Err(err) = self.swarm.behaviour_mut().gossipsub.publish(topic, vec) {
                    error!("Error publishing message: {}", err);
                }
            }
            Err(err) => {
                error!("Error serializing message: {}", err);
            }
        }
    }

    //Just logging
    #[allow(clippy::too_many_lines)]
    fn process_other_swarm_events<E>(swarm_event: SwarmEvent<GroupBehaviourEvent, E>) {
        match swarm_event {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
            } => {
                trace!("Connection established: peer_id:{:?}, endpoint:{:?}, num_established:{:?}, concurrent_dial_errors:{:?}, established_in:{:?}", peer_id, endpoint, num_established, concurrent_dial_errors, established_in);
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause: _,
            } => {
                trace!(
                    "Connection closed: peer_id:{:?}, endpoint:{:?}, num_established:{:?}",
                    peer_id,
                    endpoint,
                    num_established
                );
            }
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => {
                trace!(
                    "Incoming connection: local_addr:{:?}, send_back_addr:{:?}",
                    local_addr,
                    send_back_addr
                );
            }
            SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
            } => {
                trace!(
                    "Incoming connection error: local_addr:{:?}, send_back_addr:{:?}, error:{:?}",
                    local_addr,
                    send_back_addr,
                    error
                );
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                trace!(
                    "Outgoing connection error: peer_id:{:?}, error:{:?}",
                    peer_id,
                    error
                );
            }
            #[allow(deprecated)]
            SwarmEvent::BannedPeer { peer_id, endpoint } => {
                trace!(
                    "Banned peer: peer_id:{:?}, endpoint:{:?}",
                    peer_id,
                    endpoint
                );
            }
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                trace!(
                    "New listen address: listener_id:{:?}, address:{:?}",
                    listener_id,
                    address
                );
            }
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => {
                trace!(
                    "Expired listen address: listener_id:{:?}, address:{:?}",
                    listener_id,
                    address
                );
            }
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                trace!(
                    "Listener closed: listener_id:{:?}, addresses:{:?}, reason:{:?}",
                    listener_id,
                    addresses,
                    reason
                );
            }
            SwarmEvent::ListenerError { listener_id, error } => {
                trace!(
                    "Listener error: listener_id:{:?}, error:{:?}",
                    listener_id,
                    error
                );
            }
            SwarmEvent::Dialing(peer_id) => {
                trace!("Dialing: {peer_id:?}",);
            }

            SwarmEvent::Behaviour(_) => {
                trace!("Unexpected behaviour event");
            }
        }
    }
}
