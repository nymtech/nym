use bytes::{Buf, BufMut, BytesMut};
use log::{debug, trace};

use super::error::KcpError;

pub const KCP_HEADER: usize = 24;

/// Typed enumeration for KCP commands.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KcpCommand {
    Push = 81, // cmd: push data
    Ack = 82,  // cmd: ack
    Wask = 83, // cmd: window probe (ask)
    Wins = 84, // cmd: window size (tell)
}

impl std::fmt::Display for KcpCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KcpCommand::Push => write!(f, "Push"),
            KcpCommand::Ack => write!(f, "Ack"),
            KcpCommand::Wask => write!(f, "Window Probe (ask)"),
            KcpCommand::Wins => write!(f, "Window Size (tell)"),
        }
    }
}

impl TryFrom<u8> for KcpCommand {
    type Error = KcpError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            81 => Ok(KcpCommand::Push),
            82 => Ok(KcpCommand::Ack),
            83 => Ok(KcpCommand::Wask),
            84 => Ok(KcpCommand::Wins),
            _ => Err(KcpError::InvalidCommand(value)),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<u8> for KcpCommand {
    fn into(self) -> u8 {
        self as u8
    }
}

/// A single KCP packet (on-wire format).
#[derive(Debug, Clone)]
pub struct KcpPacket {
    conv: u32,
    cmd: KcpCommand,
    frg: u8,
    wnd: u16,
    ts: u32,
    sn: u32,
    una: u32,
    data: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
impl KcpPacket {
    pub fn new(
        conv: u32,
        cmd: KcpCommand,
        frg: u8,
        wnd: u16,
        ts: u32,
        sn: u32,
        una: u32,
        data: Vec<u8>,
    ) -> Self {
        Self {
            conv,
            cmd,
            frg,
            wnd,
            ts,
            sn,
            una,
            data,
        }
    }

    pub fn command(&self) -> KcpCommand {
        self.cmd
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn clone_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn conv(&self) -> u32 {
        self.conv
    }

    pub fn cmd(&self) -> KcpCommand {
        self.cmd
    }

    pub fn frg(&self) -> u8 {
        self.frg
    }

    pub fn wnd(&self) -> u16 {
        self.wnd
    }

    pub fn ts(&self) -> u32 {
        self.ts
    }

    pub fn sn(&self) -> u32 {
        self.sn
    }

    pub fn una(&self) -> u32 {
        self.una
    }
}

impl Default for KcpPacket {
    fn default() -> Self {
        // We must pick some default command, e.g. `Push`.
        // Or omit `Default` if you don't need it.
        KcpPacket {
            conv: 0,
            cmd: KcpCommand::Push,
            frg: 0,
            wnd: 0,
            ts: 0,
            sn: 0,
            una: 0,
            data: Vec::new(),
        }
    }
}

impl KcpPacket {
    /// Attempt to decode a `KcpPacket` from `src`.
    /// Returns Ok(Some(pkt)) if fully available, Ok(None) if not enough data,
    /// or Err(...) if there's an invalid command or other error.
    pub fn decode(src: &mut BytesMut) -> Result<Option<Self>, KcpError> {
        trace!("Decoding buffer with len: {}", src.len());
        if src.len() < KCP_HEADER {
            // Not enough for even the header, this is usually fine, more data will arrive
            debug!("Not enough data for header");
            return Ok(None);
        }

        // Peek into the first 28 bytes
        let mut header = &src[..KCP_HEADER];

        let conv = header.get_u32_le();
        let cmd_byte = header.get_u8();
        let frg = header.get_u8();
        let wnd = header.get_u16_le();
        let ts = header.get_u32_le();
        let sn = header.get_u32_le();
        let una = header.get_u32_le();
        let len = header.get_u32_le() as usize;

        let total_needed = KCP_HEADER + len;
        if src.len() < total_needed {
            // We don't have the full packet yet
            debug!(
                "Not enough data for packet, want {}, have {}",
                total_needed,
                src.len()
            );
            return Ok(None);
        }

        // Convert the raw u8 into our KcpCommand enum
        let cmd = KcpCommand::try_from(cmd_byte)?;

        // Now we can read out the data portion
        let data = src[KCP_HEADER..KCP_HEADER + len].to_vec();

        // Advance the buffer so it no longer contains this packet
        src.advance(total_needed);

        Ok(Some(Self {
            conv,
            cmd,
            frg,
            wnd,
            ts,
            sn,
            una,
            data,
        }))
    }

    /// Encode this packet into `dst`.
    pub fn encode(&self, dst: &mut BytesMut) {
        let total_len = KCP_HEADER + self.data.len();
        trace!("Encoding packet: {:?}, len: {}", self, total_len);
        dst.reserve(total_len);

        dst.put_u32_le(self.conv);
        dst.put_u8(self.cmd.into()); // Convert enum -> u8
        dst.put_u8(self.frg);
        dst.put_u16_le(self.wnd);
        dst.put_u32_le(self.ts);
        dst.put_u32_le(self.sn);
        dst.put_u32_le(self.una);
        dst.put_u32_le(self.data.len() as u32);
        dst.extend_from_slice(&self.data);

        trace!("Encoded packet: {:?}, len: {}", dst, dst.len());
    }
}
