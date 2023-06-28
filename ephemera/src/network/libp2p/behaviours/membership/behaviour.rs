//! # Membership behaviour.
//!
//! Ephemera `reliable broadcast` needs to know the list of peers who participate in the protocol.
//! Also it's not enough to have just the list but to make sure that they are actually online.
//! When the list of available peers changes, `reliable broadcast` needs to be notified so that it can adjust accordingly.
//!
//! This behaviour is responsible for keeping membership up to date.
//!
//! `MembersProviderFuture`: `Future<Output = membership::Result<Vec<PeerInfo>>> + Send + 'static`.
//!
//! User provides a [`MembersProviderFuture`] implementation to the [Behaviour] which is responsible for fetching the list of peers.
//!
//! [Behaviour] accepts only peers that are actually online.
//!
//! When peers become available or unavailable, [Behaviour] adjusts the list of connected peers accordingly and notifies `reliable broadcast`
//! about the membership change.
//!
//! It is configurable what `threshold` of peers(from the total list provided by [`MembersProviderFuture`]) should be available at any given time.
//! Or if just to use all peers who are online. See [`MembershipKind`] for more details.
//!
//! Ideally [`MembersProviderFuture`] can depend on a resource that gives reliable results. Some kind of registry which itself keeps track of actually online nodes.
//! As Ephemera uses only peers provided by [`MembersProviderFuture`], it depends on its accuracy.
//! At the same time it tries to be flexible and robust to handle less reliable [`MembersProviderFuture`] implementations.

// When peer gets disconnected, we try to dial it and if that fails, we update group.
// (it may connect us meanwhile).
// Although we can retry to connect to disconnected peers, it's simpler if we just assume that when
// they come online again, they will connect us.

//So the main goal is to remove offline peers from the group so that rb can make progress.

//a)when peer disconnects, we try to dial it and if that fails, we update group.
//b)when peer connects, we will update the group.

use std::future::Future;
use std::time::Duration;
use std::{
    collections::HashMap,
    collections::HashSet,
    fmt::Debug,
    task::{Context, Poll},
};

use futures_util::FutureExt;
use libp2p::core::Endpoint;
use libp2p::swarm::{ConnectionDenied, NotifyHandler, THandler};
use libp2p::{
    swarm::ToSwarm,
    swarm::{
        behaviour::ConnectionEstablished,
        dial_opts::{DialOpts, PeerCondition},
        ConnectionClosed, ConnectionId, DialFailure, FromSwarm, NetworkBehaviour, PollParameters,
        THandlerInEvent, THandlerOutEvent,
    },
    Multiaddr,
};
use libp2p_identity::PeerId;
use log::{debug, error, trace, warn};
use tokio::time;
use tokio::time::{Instant, Interval};

use crate::network::libp2p::behaviours::membership::handler::ToHandler;
use crate::network::libp2p::behaviours::membership::{Membership, MEMBERSHIP_SYNC_INTERVAL_SEC};
use crate::network::Peer;
use crate::{
    membership,
    network::{
        libp2p::behaviours::{
            membership::connections::ConnectedPeers,
            membership::protocol::ProtocolMessage,
            membership::{handler::Handler, MAX_DIAL_ATTEMPT_ROUNDS},
            membership::{MembershipKind, Memberships},
        },
        members::PeerInfo,
    },
};

/// [`MembersProviderFuture`] state when we are trying to connect to new peers.
///
/// We try to connect few times before giving up. Generally speaking an another peer is either online or offline
/// at any given time. But it has been helpful for testing when whole cluster comes up around the same time.
#[derive(Debug, Default)]
struct PendingPeersUpdate {
    /// Peers that we are haven't tried to connect to yet.
    waiting_to_dial: HashSet<PeerId>,
    /// Number of dial attempts per round.
    dial_attempts: usize,
    /// How long we wait between dial attempts.
    interval_between_dial_attempts: Option<Interval>,
}

#[derive(Debug, Default)]
struct SyncPeers {
    pending_peers: Vec<PeerId>,
}

impl SyncPeers {
    fn new(pending_peers: Vec<PeerId>) -> Self {
        Self { pending_peers }
    }
}

/// Behaviour states.
enum State {
    /// Waiting for new peers from the members provider trait.
    WaitingPeers,
    /// Trying to connect to new peers.
    WaitingDial(PendingPeersUpdate),
    /// We have finished trying to connect to new peers and going to report it.
    NotifyPeersUpdated,
    /// Notify other members that we have updated members.
    SyncPeers(SyncPeers),
}

/// Events that can be emitted by the `Behaviour`.
pub(crate) enum Event {
    /// We have received new peers from the members provider trait.
    /// We are going to try to connect to them.
    PeerUpdatePending,
    /// We have finished trying to connect to new peers and going to report it.
    PeersUpdated(HashSet<PeerId>),
    /// MembersProviderFuture reported us new peers and this set doesn't contain our local peer.
    LocalRemoved(HashSet<PeerId>),
    /// MembersProviderFuture reported us new peers and we failed to connect to enough of them.
    NotEnoughPeers(HashSet<PeerId>),
}

pub(crate) struct Behaviour<P>
where
    P: Future<Output = membership::Result<Vec<PeerInfo>>> + Send + 'static,
{
    /// All peers that are part of the current group.
    memberships: Memberships,
    /// Local peer id.
    local_peer_id: PeerId,
    /// Future that provides new peers.
    members_provider: P,
    /// Interval between requesting new peers from the members provider.
    members_provider_interval: Option<Interval>,
    /// Delay between dial attempts.
    members_provider_delay: Duration,
    /// Current behaviour state.
    state: State,
    /// Current state of all incoming and outgoing connections.
    all_connections: ConnectedPeers,
    /// Membership kind.
    membership_kind: MembershipKind,
    /// Last time we broadcast SYNC
    last_sync_time: Instant,
    /// Minimum time between members provider updates.
    minimum_time_between_sync: Duration,
}

impl<P> Behaviour<P>
where
    P: Future<Output = membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
{
    pub(crate) fn new(
        members_provider: P,
        members_provider_delay: Duration,
        local_peer_id: PeerId,
        membership_kind: MembershipKind,
    ) -> Self {
        let initial_delay = Instant::now() + Duration::from_secs(5);
        let delay = tokio::time::interval_at(initial_delay, members_provider_delay);
        Behaviour {
            memberships: Memberships::new(),
            local_peer_id,
            members_provider,
            members_provider_interval: Some(delay),
            members_provider_delay,
            state: State::WaitingPeers,
            all_connections: ConnectedPeers::default(),
            membership_kind,
            last_sync_time: Instant::now(),
            minimum_time_between_sync: Duration::from_secs(MEMBERSHIP_SYNC_INTERVAL_SEC),
        }
    }

    /// Returns the list of peers that are part of current group.
    pub(crate) fn active_peer_ids(&mut self) -> &HashSet<PeerId> {
        self.memberships.current().connected_peers()
    }

    pub(crate) fn active_peer_ids_with_local(&mut self) -> HashSet<PeerId> {
        self.memberships.current().connected_peer_ids_with_local()
    }

    fn waiting_peers(&mut self, cx: &mut Context) -> Poll<ToSwarm<Event, ToHandler>> {
        if let Some(mut tick) = self.members_provider_interval.take() {
            if !tick.poll_tick(cx).is_ready() {
                self.members_provider_interval = Some(tick);
                return Poll::Pending;
            }
        }
        let peers = match self.members_provider.poll_unpin(cx) {
            Poll::Ready(peers) => {
                let wait_time = Instant::now() + self.members_provider_delay;
                self.members_provider_interval =
                    time::interval_at(wait_time, self.members_provider_delay).into();
                self.last_sync_time = Instant::now();
                peers
            }
            Poll::Pending => {
                return Poll::Pending;
            }
        };

        match peers {
            Ok(peers) => {
                if peers.is_empty() {
                    //Not sure what to do here. Tempted to think that if this happens
                    //we should ignore it and assume that this is probably a bug in the membership service.

                    warn!("Received empty peers from provider. To try again before preconfigured interval, please restart the node.");
                    return Poll::Ready(ToSwarm::GenerateEvent(Event::NotEnoughPeers(
                        HashSet::default(),
                    )));
                }

                let mut new_peers = HashMap::new();

                for peer_info in peers {
                    match <PeerInfo as TryInto<Peer>>::try_into(peer_info) {
                        Ok(peer) => {
                            debug!(
                                "Received peer: {:?}, {:?}",
                                peer.peer_id.inner(),
                                peer.cosmos_address
                            );
                            new_peers.insert(*peer.peer_id.inner(), peer);
                        }
                        Err(err) => {
                            error!("Error while converting peer info to peer: {}", err);
                        }
                    }
                }

                //If we are not part of the new membership, notify immediately
                if !new_peers.contains_key(&self.local_peer_id) {
                    debug!(
                        "Local peer {:?} is not part of the new membership. Notifying immediately.",
                        self.local_peer_id
                    );
                    let pending_membership = Membership::new(new_peers.clone());
                    self.memberships.set_pending(pending_membership);
                    self.state = State::NotifyPeersUpdated;
                    return Poll::Pending;
                }

                let mut pending_membership =
                    Membership::new_with_local(new_peers.clone(), self.local_peer_id);
                let mut pending_update = PendingPeersUpdate::default();

                for peer_id in new_peers.keys() {
                    if self.all_connections.is_peer_connected(peer_id) {
                        pending_membership.peer_connected(*peer_id);
                    } else {
                        pending_update.waiting_to_dial.insert(*peer_id);
                    }
                }

                self.memberships.set_pending(pending_membership);

                //It seems that all peers from updated membership set are already connected
                if pending_update.waiting_to_dial.is_empty() {
                    self.state = State::NotifyPeersUpdated;
                    Poll::Pending
                } else {
                    self.state = State::WaitingDial(pending_update);

                    //Just let the rest of the system to know that we are in the middle of updating membership
                    Poll::Ready(ToSwarm::GenerateEvent(Event::PeerUpdatePending))
                }
            }
            Err(err) => {
                error!("Error while getting peers from provider: {:?}", err);
                Poll::Ready(ToSwarm::GenerateEvent(Event::NotEnoughPeers(
                    HashSet::default(),
                )))
            }
        }
    }

    fn waiting_dial(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Event, THandlerInEvent<Self>>> {
        if let State::WaitingDial(PendingPeersUpdate {
            waiting_to_dial,
            dial_attempts,
            interval_between_dial_attempts,
        }) = &mut self.state
        {
            //Refresh the list of connected peers
            for peer_id in self.all_connections.all_connected_peers_ref() {
                waiting_to_dial.remove(peer_id);
            }

            //FIXME
            waiting_to_dial.remove(&self.local_peer_id);

            let pending_membership = self
                .memberships
                .pending()
                .expect("Pending membership should be set");

            //With each 'poll' we can tell Swarm to dial one peer
            let next_waiting = waiting_to_dial.iter().next().copied();

            if let Some(peer_id) = next_waiting {
                waiting_to_dial.remove(&peer_id);

                let address = pending_membership
                    .peer_address(&peer_id)
                    .expect("Peer should exist");

                trace!("Dialing peer: {:?} {:?}", peer_id, address);

                let opts = DialOpts::peer_id(peer_id)
                    .condition(PeerCondition::NotDialing)
                    .addresses(vec![address.clone()])
                    .build();

                Poll::Ready(ToSwarm::Dial { opts })
            } else {
                let all_peers = pending_membership.all_peer_ids();
                let connected_peers = pending_membership.connected_peers();

                //Exclude local peer
                let all_connected = connected_peers.len() == all_peers.len() - 1;

                if all_connected || *dial_attempts >= MAX_DIAL_ATTEMPT_ROUNDS {
                    interval_between_dial_attempts.take();
                    self.state = State::NotifyPeersUpdated;
                    return Poll::Pending;
                }
                //Try again few times before notifying the rest of the system about membership update.
                //Dialing attempt
                if let Some(interval) = interval_between_dial_attempts {
                    if interval.poll_tick(cx) == Poll::Pending {
                        return Poll::Pending;
                    }
                    *dial_attempts += 1;
                    trace!("Next attempt({dial_attempts:?}) to dial failed peers");
                } else {
                    let start_at = Instant::now() + Duration::from_secs(5);
                    *interval_between_dial_attempts =
                        Some(time::interval_at(start_at, Duration::from_secs(10)));
                }
                if *dial_attempts > 0 {
                    waiting_to_dial.extend(all_peers.difference(connected_peers).copied());
                }

                Poll::Pending
            }
        } else {
            unreachable!()
        }
    }

    fn notify_peers_updated(&mut self) -> Poll<ToSwarm<Event, ToHandler>> {
        if let Some(membership) = self.memberships.remove_pending() {
            self.memberships.update(membership);
        }

        let membership = self.memberships.current();
        let membership_connected_peers = membership.connected_peer_ids();

        let event = if membership.includes_local() {
            if self.membership_kind.accept(membership) {
                debug!("Membership accepted by kind: {:?}", self.membership_kind);
                Event::PeersUpdated(membership_connected_peers)
            } else {
                debug!("Membership rejected by kind: {:?}", self.membership_kind);
                Event::NotEnoughPeers(membership_connected_peers)
            }
        } else {
            debug!("Membership does not include local peer");
            Event::LocalRemoved(membership_connected_peers)
        };

        //TODO: this list should also include "old" peers(peers who aren't part of new membership).
        let connected_peers = membership
            .connected_peer_ids()
            .into_iter()
            .collect::<Vec<_>>();
        self.state = State::SyncPeers(SyncPeers::new(connected_peers));
        Poll::Ready(ToSwarm::GenerateEvent(event))
    }

    fn sync_peers(&mut self) -> Poll<ToSwarm<Event, ToHandler>> {
        if let State::SyncPeers(SyncPeers { pending_peers }) = &mut self.state {
            match pending_peers.pop() {
                None => {
                    self.state = State::WaitingPeers;
                    Poll::Pending
                }
                Some(peer_id) => {
                    debug!("Notifying {peer_id:?} about membership update",);
                    Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: ToHandler::Message(ProtocolMessage::Sync),
                    })
                }
            }
        } else {
            unreachable!("State should be SyncPeers")
        }
    }
}

impl<P> NetworkBehaviour for Behaviour<P>
where
    P: Future<Output = membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
{
    type ConnectionHandler = Handler;
    type OutEvent = Event;

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        //TODO: we can refuse connections from peers that are not part of the current membership.
        Ok(())
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        trace!("Established inbound connection with peer: {:?}", peer);
        Ok(Handler::new())
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        _addresses: &[Multiaddr],
        _effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        //FIXME: deprecated
        #[allow(deprecated)]
        match maybe_peer {
            Some(peer_id) => Ok(self.addresses_of_peer(&peer_id)),
            None => Ok(vec![]),
        }
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        trace!(
            "Established outbound connection with peer: {:?} {:?}",
            peer,
            addr
        );
        Ok(Handler::new())
    }

    ///Membership behaviour is responsible for providing addresses to another Swarm behaviours.
    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.memberships
            .current()
            .peer_address(peer_id)
            .cloned()
            .map_or(vec![], |addr| vec![addr])
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        match event {
            FromSwarm::ConnectionEstablished(ConnectionEstablished {
                peer_id,
                connection_id: _,
                endpoint,
                failed_addresses: _,
                other_established: _,
            }) => {
                self.all_connections
                    .insert(peer_id, endpoint.clone().into());
                if let Some(pending) = self.memberships.pending_mut() {
                    pending.peer_connected(peer_id);
                }
                trace!("{:?}", self.all_connections);
            }

            FromSwarm::ConnectionClosed(ConnectionClosed {
                peer_id,
                connection_id: _,
                endpoint,
                handler: _h,
                remaining_established: _,
            }) => {
                self.all_connections
                    .remove(&peer_id, &endpoint.clone().into());
                if let Some(pending) = self.memberships.pending_mut() {
                    pending.peer_disconnected(&peer_id);
                }
                debug!("{:?}", self.all_connections);
            }
            FromSwarm::DialFailure(DialFailure {
                peer_id: Some(peer_id),
                error,
                connection_id: _,
            }) => {
                trace!("Dial failure: {:?} {:?}", peer_id, error);
            }
            _ => {}
        }
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        trace!(
            "Received event from connection handler: {:?} from peer: {:?}",
            event,
            peer_id
        );

        //TODO: we may need to check who sent the update: probably we should accept only updates from members who we already know
        if let State::WaitingPeers = self.state {
            if self.last_sync_time + self.minimum_time_between_sync < Instant::now() {
                self.members_provider_interval = None;
                debug!("Received sync notification from peer {peer_id:?}, requesting membership update");
            }
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<ToSwarm<Self::OutEvent, THandlerInEvent<Self>>> {
        match &mut self.state {
            State::WaitingPeers => self.waiting_peers(cx),
            State::WaitingDial(_) => self.waiting_dial(cx),
            State::NotifyPeersUpdated => self.notify_peers_updated(),
            State::SyncPeers(_) => self.sync_peers(),
        }
    }
}
