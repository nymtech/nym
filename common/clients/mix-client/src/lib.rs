// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use log::*;
use nymsphinx::SphinxPacket;
use std::net::SocketAddr;
use tokio::prelude::*;

pub mod packet;
pub mod poisson;

pub struct MixClient {}

impl MixClient {
    #[allow(clippy::new_without_default)]
    pub fn new() -> MixClient {
        MixClient {}
    }

    // Sends a Sphinx packet to a mixnode.
    pub async fn send(
        &self,
        packet: SphinxPacket,
        mix_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = packet.to_bytes();
        debug!("Sending to the following address: {:?}", mix_addr);

        let mut stream = tokio::net::TcpStream::connect(mix_addr).await?;
        stream.write_all(&bytes[..]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod sending_a_sphinx_packet {
    // use super::*;
    // use sphinx::SphinxPacket;

    #[test]
    fn works() {
        // arrange
        //        let directory = Client::new();
        //        let message = "Hello, Sphinx!".as_bytes().to_vec();
        //        let mixes = directory.get_mixes();
        //        let destination = directory.get_destination();
        //        let delays = sphinx::header::delays::generate(2);
        //        let packet = SphinxPacket::new(message, &mixes, &destination, &delays).unwrap();
        //        let mix_client = MixClient::new();
        //        let first_hop = mixes.first().unwrap();
        //
        //        // act
        //        mix_client.send(packet, first_hop);

        // assert
        // wtf are we supposed to assert here?
    }
}
