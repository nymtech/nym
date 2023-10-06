//! Because each Ephemera instance requests peers at arbitrary time, a node needs to notify other
//! peers when it has just requested an update. That helps to keep the whole cluster in sync and avoid
//! nodes' membership diverging.
//!
//! Overall this synchronizes all nodes' view of the membership.
//!
//! Current approach is a bit 'burst'. It makes all nodes to request membership info at the same time.
//!
//! TODO
//! Because we actually can verify peers' membership, it would be possible that one peer(or subset of peers) requests the
//! peers from a rendezvous point and then sends the list to the other peers. Or possibly only the difference.
use std::pin::Pin;
use std::task::{Context, Poll};

use asynchronous_codec::Framed;
use futures::Sink;
use futures_util::StreamExt;
use libp2p::{
    swarm::handler::{
        DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound, ListenUpgradeError,
    },
    swarm::NegotiatedSubstream,
    swarm::{
        handler::ConnectionEvent, ConnectionHandler, ConnectionHandlerEvent, KeepAlive,
        SubstreamProtocol,
    },
};
use log::{debug, error};
use thiserror::Error;

use crate::network::libp2p::behaviours::membership::protocol::{
    MembershipCodec, Protocol, ProtocolMessage,
};

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("HandlerError: {0}")]
    Handler(#[from] anyhow::Error),
}

//Because we keep long lived connections, we need to restrict number of substream attempts.
//Here we don't need more than 1 because it's a 'quiet' protocol.
const MAX_SUBSTREAM_ATTEMPTS: usize = 1;

enum InboundSubstreamState {
    WaitingInput(Framed<NegotiatedSubstream, MembershipCodec>),
    Closing(Framed<NegotiatedSubstream, MembershipCodec>),
}

enum OutboundSubstreamState {
    WaitingOutput(Framed<NegotiatedSubstream, MembershipCodec>),
    PendingSend(
        Framed<NegotiatedSubstream, MembershipCodec>,
        ProtocolMessage,
    ),
    PendingFlush(Framed<NegotiatedSubstream, MembershipCodec>),
}

pub(crate) struct Handler {
    outbound_substream: Option<OutboundSubstreamState>,
    inbound_substream: Option<InboundSubstreamState>,
    send_queue: Vec<ProtocolMessage>,
    outbound_substream_establishing: bool,
    outbound_substream_attempts: usize,
    inbound_substream_attempts: usize,
}

impl Handler {
    pub(crate) fn new() -> Self {
        Self {
            outbound_substream: None,
            inbound_substream: None,
            send_queue: vec![],
            outbound_substream_establishing: false,
            outbound_substream_attempts: 0,
            inbound_substream_attempts: 0,
        }
    }

    //Process inbound stream messages
    //WAITING_INPUT
    //  - if message received, send it to behaviour
    //  - if receive error or None, close substream
    //
    //CLOSING
    //  - Wait buffer to be flushed
    //  - Close substream
    fn process_inbound_stream(
        &mut self,
        cx: &mut Context,
    ) -> Option<Poll<ConnectionHandlerEvent<Protocol, (), FromHandler, Error>>> {
        loop {
            match std::mem::take(&mut self.inbound_substream) {
                // inbound idle state
                Some(InboundSubstreamState::WaitingInput(mut substream)) => {
                    match substream.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(message))) => {
                            self.inbound_substream =
                                Some(InboundSubstreamState::WaitingInput(substream));

                            let from_handler = FromHandler::Message(message);

                            return Poll::Ready(ConnectionHandlerEvent::Custom(from_handler))
                                .into();
                        }
                        Poll::Ready(Some(Err(err))) => {
                            error!("Failed to read from substream: {err}",);
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                        }
                        Poll::Ready(None) => {
                            debug!("Inbound stream closed by remote");
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                        }
                        Poll::Pending => {
                            self.inbound_substream =
                                Some(InboundSubstreamState::WaitingInput(substream));
                            break;
                        }
                    }
                }
                Some(InboundSubstreamState::Closing(mut substream)) => {
                    match Sink::poll_close(Pin::new(&mut substream), cx) {
                        Poll::Ready(res) => {
                            if let Err(e) = res {
                                error!("Inbound substream error while closing: {e}");
                            }
                            self.inbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                            break;
                        }
                    }
                }
                None => {
                    self.inbound_substream = None;
                    break;
                }
            }
        }
        None
    }

    //Process outbound stream messages
    //WAITING_OUTPUT
    //  - if send queue is not empty, go to PENDING_SEND
    //
    //PENDING_SEND
    //  - send message to substream
    //  - if send error, mark substream as Closing
    //
    //PENDING_FLUSH
    //  - flush substream
    //  - if flush error, mark substream as Closing
    fn process_outbound_stream(&mut self, cx: &mut Context) {
        loop {
            match std::mem::take(&mut self.outbound_substream) {
                // outbound idle state
                Some(OutboundSubstreamState::WaitingOutput(substream)) => {
                    if let Some(message) = self.send_queue.pop() {
                        self.outbound_substream =
                            Some(OutboundSubstreamState::PendingSend(substream, message));
                        continue;
                    }

                    self.outbound_substream =
                        Some(OutboundSubstreamState::WaitingOutput(substream));
                    break;
                }
                Some(OutboundSubstreamState::PendingSend(mut substream, message)) => {
                    match Sink::poll_ready(Pin::new(&mut substream), cx) {
                        Poll::Ready(Ok(())) => {
                            match Sink::start_send(Pin::new(&mut substream), message) {
                                Ok(()) => {
                                    self.outbound_substream =
                                        Some(OutboundSubstreamState::PendingFlush(substream));
                                }
                                Err(e) => {
                                    debug!("Failed to send message on outbound stream: {e}");
                                    self.outbound_substream = None;
                                    break;
                                }
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            debug!("Failed to send message on outbound stream: {e}");
                            self.outbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.outbound_substream =
                                Some(OutboundSubstreamState::PendingSend(substream, message));
                            break;
                        }
                    }
                }
                Some(OutboundSubstreamState::PendingFlush(mut substream)) => {
                    match Sink::poll_flush(Pin::new(&mut substream), cx) {
                        Poll::Ready(Ok(())) => {
                            self.outbound_substream =
                                Some(OutboundSubstreamState::WaitingOutput(substream));
                        }
                        Poll::Ready(Err(e)) => {
                            debug!("Failed to flush outbound stream: {e}");
                            self.outbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.outbound_substream =
                                Some(OutboundSubstreamState::PendingFlush(substream));
                            break;
                        }
                    }
                }
                None => {
                    self.outbound_substream = None;
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum FromHandler {
    Message(ProtocolMessage),
}

#[derive(Debug)]
pub(crate) enum ToHandler {
    Message(ProtocolMessage),
}

impl ConnectionHandler for Handler {
    type InEvent = ToHandler;
    type OutEvent = FromHandler;
    type Error = Error;
    type InboundProtocol = Protocol;
    type OutboundProtocol = Protocol;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Protocol, ()> {
        SubstreamProtocol::new(Protocol, ())
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        //we could add idle timeout here
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        //  poll STATE_MACHINE
        //- Request outbound substream if neccessary
        //- poll inbound substream
        //- poll outbound substream

        //Establish new connection when behaviour wants to send a message and we don't have an outbound substream yet
        if !self.send_queue.is_empty()
            && self.outbound_substream.is_none()
            && !self.outbound_substream_establishing
        {
            self.outbound_substream_establishing = true;
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(Protocol, ()),
            });
        }

        if let Some(res) = self.process_inbound_stream(cx) {
            return res;
        }

        self.process_outbound_stream(cx);

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::InEvent) {
        match event {
            ToHandler::Message(message) => {
                self.send_queue.push(message);
            }
        }
    }

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol: stream,
                info: (),
            }) => {
                if self.inbound_substream_attempts > MAX_SUBSTREAM_ATTEMPTS {
                    log::warn!("Too many inbound substream attempts, refusing stream");
                    return;
                }
                self.inbound_substream_attempts += 1;
                self.inbound_substream = Some(InboundSubstreamState::WaitingInput(stream));
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol,
                info: (),
            }) => {
                if self.outbound_substream_attempts > MAX_SUBSTREAM_ATTEMPTS {
                    log::warn!("Too many outbound substream attempts, refusing stream");
                    return;
                }
                self.outbound_substream = Some(OutboundSubstreamState::WaitingOutput(protocol));
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError { info, error }) => {
                error!("DialUpgradeError: info: {:?}, error: {:?}", info, error);
            }
            ConnectionEvent::ListenUpgradeError(ListenUpgradeError { info, error }) => {
                error!("ListenUpgradeError: info: {:?}, error: {:?}", info, error);
            }
            ConnectionEvent::AddressChange(_) => {}
        }
    }
}
