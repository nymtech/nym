use bytes::BytesMut;
use log::{debug, trace};

use crate::{error::KcpError, packet::KcpPacket, session::KcpSession};

pub struct KcpDriver {
    session: KcpSession,
    buffer: BytesMut,
}

impl KcpDriver {
    pub fn conv_id(&self) -> Result<u32, KcpError> {
        Ok(self.session.conv)
    }

    pub fn send(&mut self, data: &[u8]) {
        self.session.send(data);
    }

    pub fn input(&mut self, data: &[u8]) -> Result<Vec<KcpPacket>, KcpError> {
        self.buffer.extend_from_slice(data);
        let mut pkts = Vec::new();
        while let Ok(Some(pkt)) = KcpPacket::decode(&mut self.buffer) {
            debug!(
                "Decoded packet, cmd: {}, sn: {}, frg: {}",
                pkt.command(),
                pkt.sn(),
                pkt.frg()
            );
            self._input(&pkt)?;
            pkts.push(pkt);
        }
        Ok(pkts)
    }

    fn _input(&mut self, pkt: &KcpPacket) -> Result<(), KcpError> {
        self.session.input(pkt)
    }

    pub fn fetch_outgoing(&mut self) -> Vec<KcpPacket> {
        trace!(
            "ts_flush: {}, ts_current: {}",
            self.session.ts_flush(),
            self.session.ts_current()
        );
        self.session.fetch_outgoing()
    }

    pub fn update(&mut self, tick: u64) {
        self.session.update(tick as u32);
    }

    pub fn new(session: KcpSession) -> Self {
        KcpDriver {
            session,
            buffer: BytesMut::new(),
        }
    }

    /// Fetch any complete messages that have been reassembled from received KCP packets.
    ///
    /// Returns a vector of complete messages. Messages are only returned once all
    /// fragments have been received and reassembled.
    pub fn fetch_incoming(&mut self) -> Vec<BytesMut> {
        self.session.fetch_incoming()
    }

    /// Read reassembled data into a buffer.
    ///
    /// Returns the number of bytes read into the buffer.
    /// If no complete message is available, returns 0.
    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        self.session.recv(buf)
    }
}
