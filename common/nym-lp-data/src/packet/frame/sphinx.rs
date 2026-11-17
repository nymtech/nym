// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_params::SphinxKeyRotation;
use nym_sphinx_params::key_rotation::InvalidSphinxKeyRotation;

use crate::packet::frame::LpFrameAttributes;

/// Parsed form of the 14-byte `frame_attributes` for [`LpFrameKind::SphinxPacket`].
///
/// Wire layout:
/// ```text
/// [0] key_rotation : u8
/// ```
///
/// The recipient works out the next hop from the packet itself, so there is nothing else to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SphinxFrameAttributes {
    pub key_rotation: SphinxKeyRotation,
}

impl TryFrom<LpFrameAttributes> for SphinxFrameAttributes {
    type Error = InvalidSphinxKeyRotation;

    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        Ok(SphinxFrameAttributes {
            key_rotation: value[0].try_into()?,
        })
    }
}

impl From<SphinxFrameAttributes> for LpFrameAttributes {
    fn from(value: SphinxFrameAttributes) -> Self {
        let mut attrs = [0; 14];
        attrs[0] = value.key_rotation as u8;
        attrs
    }
}

/// Parsed form of the 14-byte `frame_attributes` for [`LpFrameKind::ForwardSphinxPacket`].
///
/// Wire layout (big-endian):
/// ```text
/// [0    ) key_rotation : u8
/// [1..5) next_hop      : u32   (a mixnet node id)
/// ```
///
/// Unlike [`SphinxFrameAttributes`], the receiver forwards the packet without processing it, so it
/// is told where to send it. By node id, because that is what it can look up: the packet names its
/// next hop by address, which the receiver has no index for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForwardSphinxFrameAttributes {
    pub key_rotation: SphinxKeyRotation,

    /// Mixnet node id of the hop to forward to (`nym_topology::NodeId`, which `nym-lp-data` cannot
    /// name without depending on the topology).
    pub next_hop: u32,
}

impl TryFrom<LpFrameAttributes> for ForwardSphinxFrameAttributes {
    type Error = InvalidSphinxKeyRotation;

    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        let next_hop = u32::from_le_bytes([value[1], value[2], value[3], value[4]]);

        Ok(ForwardSphinxFrameAttributes {
            key_rotation: value[0].try_into()?,
            next_hop,
        })
    }
}

impl From<ForwardSphinxFrameAttributes> for LpFrameAttributes {
    fn from(value: ForwardSphinxFrameAttributes) -> Self {
        let mut attrs = [0; 14];
        attrs[0] = value.key_rotation as u8;
        attrs[1..5].copy_from_slice(&value.next_hop.to_le_bytes());
        attrs
    }
}

/// What a forwarded packet becomes for the hop it is forwarded to, which processes it rather than
/// forwarding it again and so has no use for the node id.
impl From<ForwardSphinxFrameAttributes> for SphinxFrameAttributes {
    fn from(value: ForwardSphinxFrameAttributes) -> Self {
        SphinxFrameAttributes {
            key_rotation: value.key_rotation,
        }
    }
}
