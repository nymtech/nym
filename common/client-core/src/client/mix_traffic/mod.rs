// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::sender::{MixnetSender, RemoteGateway};
use crate::spawn_future;
use log::*;
use nym_credential_storage::storage::Storage;
use nym_gateway_client::GatewayClient;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use std::marker::PhantomData;

pub type BatchMixMessageSender = tokio::sync::mpsc::Sender<Vec<MixPacket>>;
pub type BatchMixMessageReceiver = tokio::sync::mpsc::Receiver<Vec<MixPacket>>;

mod sender;

// We remind ourselves that 32 x 32kb = 1024kb, a reasonable size for a network buffer.
pub const MIX_MESSAGE_RECEIVER_BUFFER_SIZE: usize = 32;
const MAX_FAILURE_COUNT: usize = 100;

// that's also disgusting.
pub struct Empty;

pub struct MixTrafficController<C = Empty, St = Empty, S = RemoteGateway<C, St>> {
    // TODO: most likely to be replaced by some higher level construct as
    // later on gateway_client will need to be accessible by other entities
    mixnet_sender: S,

    mix_rx: BatchMixMessageReceiver,

    // TODO: this is temporary work-around.
    // in long run `gateway_client` will be moved away from `MixTrafficController` anyway.
    consecutive_gateway_failure_count: usize,

    // ugh, I hate the existence of those, but couldn't think of a way to remove it
    _phantom_dkg: PhantomData<C>,
    _phantom_storage: PhantomData<St>,
}

impl<C, St> MixTrafficController<C, St>
where
    C: DkgQueryClient + Sync + Send + 'static,
    St: Storage + 'static,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    pub fn new_remote(
        gateway_client: GatewayClient<C, St>,
    ) -> (MixTrafficController<C, St>, BatchMixMessageSender) {
        Self::new(RemoteGateway::new(gateway_client))
    }
}

impl<C, St, S> MixTrafficController<C, St, S>
where
    C: Send + 'static,
    St: Send + 'static,
    S: MixnetSender + Send + 'static,
{
    pub fn new(mixnet_sender: S) -> (MixTrafficController<C, St, S>, BatchMixMessageSender) {
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(MIX_MESSAGE_RECEIVER_BUFFER_SIZE);
        (
            MixTrafficController {
                mixnet_sender,
                mix_rx: message_receiver,
                consecutive_gateway_failure_count: 0,
                _phantom_dkg: Default::default(),
                _phantom_storage: Default::default(),
            },
            message_sender,
        )
    }

    async fn on_messages(&mut self, mut mix_packets: Vec<MixPacket>) {
        debug_assert!(!mix_packets.is_empty());

        let result = if mix_packets.len() == 1 {
            let mix_packet = mix_packets.pop().unwrap();
            self.mixnet_sender.send_mix_packet(mix_packet).await
        } else {
            self.mixnet_sender.batch_send_mix_packets(mix_packets).await
        };

        match result {
            Err(err) => {
                error!("Failed to send sphinx packet(s) to the gateway: {err}");
                self.consecutive_gateway_failure_count += 1;
                if self.consecutive_gateway_failure_count == MAX_FAILURE_COUNT {
                    // todo: in the future this should initiate a 'graceful' shutdown or try
                    // to reconnect?
                    panic!("failed to send sphinx packet to the gateway {MAX_FAILURE_COUNT} times in a row - assuming the gateway is dead. Can't do anything about it yet :(")
                }
            }
            Ok(_) => {
                trace!("We *might* have managed to forward sphinx packet(s) to the gateway!");
                self.consecutive_gateway_failure_count = 0;
            }
        }
    }

    pub fn start_with_shutdown(mut self, mut shutdown: nym_task::TaskClient) {
        spawn_future(async move {
            debug!("Started MixTrafficController with graceful shutdown support");

            loop {
                tokio::select! {
                    mix_packets = self.mix_rx.recv() => match mix_packets {
                        Some(mix_packets) => {
                            self.on_messages(mix_packets).await;
                        },
                        None => {
                            log::trace!("MixTrafficController: Stopping since channel closed");
                            break;
                        }
                    },
                    _ = shutdown.recv_with_delay() => {
                        log::trace!("MixTrafficController: Received shutdown");
                        break;
                    }
                }
            }
            shutdown.recv_timeout().await;
            log::debug!("MixTrafficController: Exiting");
        })
    }
}
