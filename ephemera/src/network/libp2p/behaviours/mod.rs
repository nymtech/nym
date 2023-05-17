use std::future::Future;
use std::{iter, sync::Arc, time::Duration};

use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    dns, gossipsub,
    gossipsub::{IdentTopic as Topic, MessageAuthenticity, ValidationMode},
    kad, noise, request_response as libp2p_request_response,
    swarm::NetworkBehaviour,
    tcp::{tokio::Transport as TokioTransport, Config as TokioConfig},
    yamux, PeerId as Libp2pPeerId, Transport,
};
use log::info;

use crate::membership::PeerInfo;
use crate::network::libp2p::behaviours::membership::MembershipKind;
use crate::{
    broadcast::RbMsg,
    crypto::Keypair,
    network::libp2p::behaviours::request_response::{
        RbMsgMessagesCodec, RbMsgProtocol, RbMsgResponse,
    },
    peer::{PeerId, ToPeerId},
    utilities::hash::{EphemeraHasher, Hasher},
};

pub(crate) mod membership;
pub(crate) mod request_response;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "GroupBehaviourEvent")]
pub(crate) struct GroupNetworkBehaviour<P>
where
    P: Future<Output = crate::membership::Result<Vec<PeerInfo>>> + Send + 'static,
{
    pub(crate) members_provider: membership::behaviour::Behaviour<P>,
    pub(crate) gossipsub: gossipsub::Behaviour,
    pub(crate) request_response: libp2p_request_response::Behaviour<RbMsgMessagesCodec>,
    pub(crate) kademlia: kad::Kademlia<kad::store::MemoryStore>,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum GroupBehaviourEvent {
    Gossipsub(gossipsub::Event),
    RequestResponse(libp2p_request_response::Event<RbMsg, RbMsgResponse>),
    Membership(membership::behaviour::Event),
    Kademlia(kad::KademliaEvent),
}

impl From<gossipsub::Event> for GroupBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        GroupBehaviourEvent::Gossipsub(event)
    }
}

impl From<libp2p_request_response::Event<RbMsg, RbMsgResponse>> for GroupBehaviourEvent {
    fn from(event: libp2p_request_response::Event<RbMsg, RbMsgResponse>) -> Self {
        GroupBehaviourEvent::RequestResponse(event)
    }
}

impl From<membership::behaviour::Event> for GroupBehaviourEvent {
    fn from(event: membership::behaviour::Event) -> Self {
        GroupBehaviourEvent::Membership(event)
    }
}

impl From<kad::KademliaEvent> for GroupBehaviourEvent {
    fn from(event: kad::KademliaEvent) -> Self {
        GroupBehaviourEvent::Kademlia(event)
    }
}

//Create combined behaviour.
//Gossipsub takes care of message delivery semantics
//Membership takes care of providing peers who are part of the reliable broadcast group
//Kademlia takes provides closest neighbours and general DHT functionality
pub(crate) fn create_behaviour<P>(
    keypair: &Arc<Keypair>,
    ephemera_msg_topic: &Topic,
    members_provider: P,
    members_provider_delay: Duration,
    membership_kind: MembershipKind,
) -> GroupNetworkBehaviour<P>
where
    P: Future<Output = crate::membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
{
    //TODO: review behaviours config(eg. gossipsub minimum peers, kademlia ttl, request-response timeouts etc.)
    let local_peer_id = keypair.peer_id();
    let gossipsub = create_gossipsub(keypair, ephemera_msg_topic);
    let request_response = create_request_response();
    let rendezvous_behaviour = create_membership(
        members_provider,
        members_provider_delay,
        membership_kind,
        local_peer_id,
    );
    let kademlia = create_kademlia(keypair);

    GroupNetworkBehaviour {
        members_provider: rendezvous_behaviour,
        gossipsub,
        request_response,
        kademlia,
    }
}

// Configure networking messaging stack(Gossipsub)
pub(crate) fn create_gossipsub(local_key: &Arc<Keypair>, topic: &Topic) -> gossipsub::Behaviour {
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        //TODO: settings from config
        .heartbeat_interval(Duration::from_secs(5))
        .message_id_fn(|msg: &gossipsub::Message| Hasher::digest(&msg.data).into())
        .validation_mode(ValidationMode::Strict)
        .build()
        .expect("Valid config");

    let mut behaviour = gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(local_key.inner().clone()),
        gossipsub_config,
    )
    .expect("Correct configuration");

    info!("Subscribing to topic: {}", topic);
    behaviour.subscribe(topic).expect("Valid topic");
    behaviour
}

pub(crate) fn create_request_response() -> libp2p_request_response::Behaviour<RbMsgMessagesCodec> {
    let config = libp2p_request_response::Config::default();
    libp2p_request_response::Behaviour::new(
        RbMsgMessagesCodec,
        iter::once((
            RbMsgProtocol,
            libp2p_request_response::ProtocolSupport::Full,
        )),
        config,
    )
}

pub(crate) fn create_membership<P>(
    members_provider: P,
    members_provider_delay: Duration,
    membership_kind: MembershipKind,
    local_peer_id: PeerId,
) -> membership::behaviour::Behaviour<P>
where
    P: Future<Output = crate::membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
{
    membership::behaviour::Behaviour::new(
        members_provider,
        members_provider_delay,
        local_peer_id.into(),
        membership_kind,
    )
}

pub(super) fn create_kademlia(local_key: &Arc<Keypair>) -> kad::Kademlia<kad::store::MemoryStore> {
    let peer_id = local_key.peer_id();
    let mut cfg = kad::KademliaConfig::default();
    cfg.set_query_timeout(Duration::from_secs(5 * 60));
    let store = kad::store::MemoryStore::new(peer_id.0);
    kad::Kademlia::with_config(*peer_id.inner(), store, cfg)
}

//Configure networking connection stack(Tcp, Noise, Yamux)
//Tcp protocol for networking
//Noise protocol for encryption
//Yamux protocol for multiplexing
pub(crate) fn create_transport(
    local_key: &Arc<Keypair>,
) -> anyhow::Result<Boxed<(Libp2pPeerId, StreamMuxerBox)>> {
    let transport = TokioTransport::new(TokioConfig::default().nodelay(true));
    let transport = dns::TokioDnsConfig::system(transport)?;

    let noise_config = noise::Config::new(local_key.inner())?;
    Ok(transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed())
}
