use std::convert::TryFrom;

// it's up to the smart people to figure those values out : )
const REGULAR_PACKET_SIZE: usize = 2 * 1024;
const ACK_PACKET_SIZE: usize = 512;
const EXTENDED_PACKET_SIZE: usize = 32 * 1024;

pub struct InvalidPacketSize;

#[repr(u8)]
pub enum PacketSize {
    RegularPacket = 1,  // for example instant messaging use case
    ACKPacket = 2,      // for sending SURB-ACKs
    ExtendedPacket = 3, // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality

    PreSURBChanges = 0,
}

impl TryFrom<u8> for PacketSize {
    type Error = InvalidPacketSize;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == (PacketSize::RegularPacket as u8) => Ok(Self::RegularPacket),
            _ if value == (PacketSize::ACKPacket as u8) => Ok(Self::ACKPacket),
            _ if value == (PacketSize::ExtendedPacket as u8) => Ok(Self::ExtendedPacket),
            _ if value == (PacketSize::PreSURBChanges as u8) => Ok(Self::PreSURBChanges),
            _ => Err(InvalidPacketSize),
        }
    }
}

impl PacketSize {
    pub fn size(&self) -> usize {
        match &self {
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            PacketSize::ACKPacket => ACK_PACKET_SIZE,
            PacketSize::ExtendedPacket => EXTENDED_PACKET_SIZE,
            PacketSize::PreSURBChanges => crate::PACKET_SIZE,
        }
    }

    pub fn get_type(size: usize) -> std::result::Result<Self, InvalidPacketSize> {
        if PacketSize::RegularPacket.size() == size {
            Ok(PacketSize::RegularPacket)
        } else if PacketSize::ACKPacket.size() == size {
            Ok(PacketSize::ACKPacket)
        } else if PacketSize::ExtendedPacket.size() == size {
            Ok(PacketSize::ExtendedPacket)
        } else if PacketSize::PreSURBChanges.size() == size {
            Ok(PacketSize::PreSURBChanges)
        } else {
            Err(InvalidPacketSize)
        }
    }
}
