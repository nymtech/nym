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

// re-exporting types and constants available in sphinx
pub use sphinx::{
    constants::{
        self, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, MAXIMUM_PLAINTEXT_LENGTH,
        MAX_PATH_LENGTH, NODE_ADDRESS_LENGTH,
    },
    header::{self, delays, delays::Delay, ProcessedHeader, SphinxHeader, HEADER_SIZE},
    payload::Payload,
    route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier},
    Error, ProcessedPacket, Result, SphinxPacket, PACKET_SIZE,
};

// re-exporting this separately to remember to put special attention to below
// modules/types/constants when refactoring sphinx crate itself
// TODO: replace with sphinx::PublicKey once merged
pub use sphinx::key;
