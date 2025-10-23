use std::{
    cmp,
    collections::VecDeque,
    io::{self, Read, Write},
};

use ansi_term::Color::Yellow;
use bytes::{Buf, BytesMut};
use log::{debug, error, info, warn};
use std::thread;

use super::packet::{KcpCommand, KcpPacket};

/// Minimal KCP session that produces/consumes `KcpPacket`s
pub struct KcpSession {
    pub conv: u32,

    // Basic send parameters
    snd_nxt: u32, // next sequence to send
    snd_una: u32, // first unacknowledged
    snd_wnd: u16, // local send window
    rmt_wnd: u16, // remote receive window (from packets)
    snd_queue: VecDeque<Segment>,
    snd_buf: VecDeque<Segment>,

    // Basic receive parameters
    rcv_nxt: u32, // next sequence expected
    rcv_wnd: u16, // local receive window
    rcv_buf: VecDeque<Segment>,
    rcv_queue: VecDeque<BytesMut>,

    // RTT calculation
    rx_srtt: u32,
    rx_rttval: u32,
    rx_rto: u32,
    rx_minrto: u32,

    // Timers
    current: u32,  // current clock (ms)
    interval: u32, // flush interval
    ts_flush: u32, // next flush timestamp

    // If you want to store outgoing packets from flush, do it here
    out_pkts: Vec<KcpPacket>,
    mtu: usize,
    partial_read: Option<BytesMut>,
}

/// Internal segment type: similar to `KcpPacket` but includes metadata for retransmissions.
#[derive(Debug, Clone)]
struct Segment {
    sn: u32,
    frg: u8,
    ts: u32,
    resendts: u32,
    rto: u32,
    xmit: u32, // how many times sent
    data: Vec<u8>,
}

impl Segment {
    #[allow(dead_code)]
    fn new(sn: u32, frg: u8, data: Vec<u8>) -> Self {
        Segment {
            sn,
            frg,
            ts: 0,
            resendts: 0,
            rto: 0,
            xmit: 0,
            data,
        }
    }
}

impl Default for KcpSession {
    fn default() -> Self {
        KcpSession {
            conv: 0,
            snd_nxt: 0,
            snd_una: 0,
            snd_wnd: 32,
            rmt_wnd: 128,
            snd_queue: VecDeque::new(),
            snd_buf: VecDeque::new(),

            rcv_nxt: 0,
            rcv_wnd: 128,
            rcv_buf: VecDeque::new(),
            rcv_queue: VecDeque::new(),

            rx_srtt: 0,
            rx_rttval: 0,
            rx_rto: 3000,
            rx_minrto: 3000,

            current: 0,
            interval: 100,
            ts_flush: 100,

            out_pkts: Vec::new(),
            mtu: 1376,
            partial_read: None,
        }
    }
}

impl KcpSession {
    pub fn ts_current(&self) -> u32 {
        self.current
    }

    pub fn ts_flush(&self) -> u32 {
        self.ts_flush
    }

    fn available_send_segments(&self) -> usize {
        // A naive approach: if `snd_queue` has length L
        // and local window is `snd_wnd`, we can add `snd_wnd - L` more segments
        let used = self.snd_queue.len();
        let allowed = self.snd_wnd as usize;
        allowed.saturating_sub(used)
    }

    /// Create a new KCP session with a specified conv ID and default MSS.
    pub fn new(conv: u32) -> Self {
        KcpSession {
            conv,
            ..Default::default()
        }
    }

    /// If you want to let the user set the mtu:
    pub fn set_mtu(&mut self, mtu: usize) {
        self.mtu = mtu;
    }

    /// Set the update interval (flush interval) in milliseconds
    pub fn set_interval(&mut self, interval: u32) {
        let interval = interval.clamp(10, 5000);
        self.interval = interval;
    }

    /// Manually set the minimal RTO
    pub fn set_min_rto(&mut self, rto: u32) {
        self.rx_minrto = rto;
    }

    pub fn send(&mut self, mut data: &[u8]) {
        debug!("Sending data, len: {}", data.len());

        if data.is_empty() {
            return;
        }

        // How many segments do we need?
        // If data <= mss, it's 1; otherwise multiple.
        let total_len = data.len();
        let count = if total_len <= self.mtu {
            1
        } else {
            total_len.div_ceil(self.mtu)
        };

        debug!("Will send {} fragments", count);

        // Build each fragment
        for i in 0..count {
            let size = std::cmp::min(self.mtu, data.len());
            let chunk = &data[..size];

            // KCP fragment numbering is REVERSED - last fragment has frg=0,
            // first has frg=count-1. This allows receiver to know total count from first packet.
            // In KCP, `frg` is set to the remaining fragments in reverse order.
            // i.e., the last fragment has frg=0, the first has frg=count-1.
            let frg = (count - i - 1) as u8;

            let seg = Segment {
                sn: self.snd_nxt,
                frg,
                ts: 0,
                resendts: 0,
                rto: 0,
                xmit: 0,
                data: chunk.to_vec(),
            };

            debug!("Sending segment, sn: {}, frg: {}", seg.sn, seg.frg);

            self.snd_queue.push_back(seg);
            debug!("snd_queue len: {}", self.snd_queue.len());

            self.snd_nxt = self.snd_nxt.wrapping_add(1);

            // Advance the slice
            data = &data[size..];

            debug!("Remaining data, len: {}", data.len());
        }
    }

    /// Input a newly received packet from the network (after decryption).
    pub fn input(&mut self, pkt: &KcpPacket) {
        debug!(
            "[ConvID: {}, Thread: {:?}] input: Received packet - cmd: {:?}, sn: {}, frg: {}, wnd: {}, ts: {}, una: {}",
            self.conv,
            thread::current().id(),
            pkt.cmd(),
            pkt.sn(),
            pkt.frg(),
            pkt.wnd(),
            pkt.ts(),
            pkt.una()
        );

        // Check conv
        if pkt.conv() != self.conv {
            error!(
                "Received packet with wrong conv: {} != {}",
                pkt.conv(),
                self.conv
            );
            return;
        }

        // Update remote window
        self.rmt_wnd = pkt.wnd();

        // Parse UNA first - crucial for clearing snd_buf before processing ACKs/data
        self.parse_una(pkt.una());

        // Log snd_buf state before specific command processing
        let pre_cmd_sns: Vec<u32> = self.snd_buf.iter().map(|seg| seg.sn).collect();
        debug!(
            "[ConvID: {}, Thread: {:?}] input: Pre-command processing snd_buf (len={}): {:?}",
            self.conv,
            thread::current().id(),
            self.snd_buf.len(),
            pre_cmd_sns
        );

        match pkt.cmd() {
            KcpCommand::Ack => {
                self.parse_ack(pkt.sn(), pkt.ts());
            }
            KcpCommand::Push => {
                debug!("Received push, sn: {}, frg: {}", pkt.sn(), pkt.frg());
                // Data
                // self.ack_push(pkt.sn(), self.current); // Send ack eventually
                self.ack_push(pkt.sn(), pkt.ts());
                self.parse_data(pkt);
            }
            KcpCommand::Wask => {
                error!("Received window probe, this is unimplemented");
                // Window probe from remote -> we'll respond with Wins
                // Not implemented in this minimal
            }
            KcpCommand::Wins => {
                error!("Received window size, this is unimplemented");
                // Remote sends window size
                // Not implemented in this minimal
            }
        }
    }

    /// Update KCP state with `delta_ms` since the last call.
    /// This increments `current` by `delta_ms` and performs any flushing logic if needed.
    pub fn update(&mut self, delta_ms: u32) {
        // 1) Advance our "current time" by delta_ms
        self.current = self.current.saturating_add(delta_ms);

        // 2) Check if it's time to flush
        if !self.should_flush() {
            // not yet time to flush
            return;
        }

        self.ts_flush += self.interval;
        if self.ts_flush < self.current {
            self.ts_flush = self.current + self.interval;
        }

        // 3) Move segments from snd_queue -> snd_buf if window allows
        // debug!("send queue len: {}", self.snd_queue.len());
        self.move_queue_to_buf();
        // debug!("send buf len: {}", self.snd_buf.len());
        // 4) Check for retransmissions, produce outgoing packets
        self.flush_outgoing();
        // debug!("send buf len: {}", self.snd_buf.len());
    }

    /// Retrieve any newly created packets that need sending (e.g., data or ack).
    /// After calling `update`, call this to get the `KcpPacket`s. Then you can
    /// encrypt them and actually write them out (UDP, file, etc.).
    pub fn fetch_outgoing(&mut self) -> Vec<KcpPacket> {
        let mut result = Vec::new();
        std::mem::swap(&mut result, &mut self.out_pkts); // take ownership
        result
    }

    pub fn fetch_incoming(&mut self) -> Vec<BytesMut> {
        let mut result = Vec::new();
        while let Some(message) = self.rcv_queue.pop_front() {
            result.push(message);
        }
        result
    }

    pub fn recv(&mut self, out: &mut [u8]) -> usize {
        if out.is_empty() {
            return 0;
        }

        let mut read_bytes = 0;

        // 1) If there's leftover partial data, read from that first
        if let Some(ref mut leftover) = self.partial_read {
            let to_copy = std::cmp::min(out.len(), leftover.len());
            out[..to_copy].copy_from_slice(&leftover[..to_copy]);
            read_bytes += to_copy;
            // Remove the consumed portion from leftover
            leftover.advance(to_copy);

            if leftover.is_empty() {
                // If we've exhausted the leftover, clear it
                self.partial_read = None;
            }

            // If we've already filled 'out', return
            if read_bytes == out.len() {
                return read_bytes;
            }
        }

        // 2) If we still have space, consume messages from rcv_queue
        while read_bytes < out.len() {
            // If there's no complete message left, break
            let mut msg = match self.rcv_queue.pop_front() {
                None => break,
                Some(m) => m,
            };

            let space_left = out.len() - read_bytes;
            if msg.len() <= space_left {
                // The entire message fits into 'out'
                out[read_bytes..read_bytes + msg.len()].copy_from_slice(&msg);
                read_bytes += msg.len();
            } else {
                // msg is larger than what's left in 'out'
                out[read_bytes..].copy_from_slice(&msg[..space_left]);
                read_bytes += space_left;

                // Keep the leftover part of 'msg' in partial_read
                msg.advance(space_left);
                self.partial_read = Some(msg);

                // We've filled 'out', so stop
                break;
            }
        }

        read_bytes
    }

    //---------------------------------------------------------------------------------
    // Internal methods

    fn should_flush(&self) -> bool {
        // flush if current >= ts_flush
        // or if we've never updated
        self.current >= self.ts_flush
    }

    /// Move segments from `snd_queue` into `snd_buf` respecting window
    fn move_queue_to_buf(&mut self) {
        // Calculate the congestion window (cwnd)
        let cwnd = std::cmp::min(self.snd_wnd, self.rmt_wnd);

        // In real KCP, we check against the number of unacknowledged segments:
        // while self.snd_nxt < self.snd_una + cwnd { ... }
        // Here, we approximate by checking the current length of snd_buf against cwnd.
        while let Some(mut seg) = self.snd_queue.pop_front() {
            // Check if adding this segment would exceed the congestion window
            if (self.snd_buf.len() as u16) >= cwnd {
                // Effective window is full
                self.snd_queue.push_front(seg); // Put it back
                break;
            }
            // init rto
            seg.xmit = 0;
            seg.rto = self.rx_rto;
            seg.resendts = 0; // will set later
            seg.ts = self.current;
            self.snd_buf.push_back(seg);
        }
    }

    /// Build KcpPacket(s) for segments needing send or retransmit.
    fn flush_outgoing(&mut self) {
        // Log current snd_buf state before iterating
        // let current_sns: Vec<u32> = self.snd_buf.iter().map(|seg| seg.sn).collect();
        // debug!(
        //     "[ConvID: {}, Thread: {:?}] flush_outgoing: Checking snd_buf (len={}): {:?}",
        //     self.conv,
        //     thread::current().id(),
        //     self.snd_buf.len(),
        //     current_sns
        // );

        for seg in &mut self.snd_buf {
            let mut need_send = false;
            if seg.xmit == 0 {
                // never sent
                need_send = true;
                seg.xmit = 1;
                seg.resendts = self.current + seg.rto;
            } else if self.current >= seg.resendts {
                // time to retransmit
                need_send = true;
                seg.xmit += 1;
                // Exponential backoff: double RTO for this segment
                seg.rto *= 2;
                // Clamp to the session's maximum RTO (hardcoded as 60s for now)
                const MAX_RTO: u32 = 60000; // Same as used in update_rtt
                if seg.rto > MAX_RTO {
                    seg.rto = MAX_RTO;
                }
                seg.resendts = self.current + seg.rto;
                info!(
                    "{}",
                    Yellow.paint(format!(
                        "Retransmit conv_id: {}, sn: {}, frg: {}",
                        self.conv, seg.sn, seg.frg
                    ))
                );
            }

            if need_send {
                // Make a KcpPacket
                let pkt = KcpPacket::new(
                    self.conv,
                    KcpCommand::Push,
                    seg.frg,
                    self.rcv_wnd,
                    seg.ts, // original send timestamp
                    seg.sn,
                    self.rcv_nxt, // self.rcv_nxt for ack
                    seg.data.clone(),
                );
                self.out_pkts.push(pkt);

                // if too many xmit => dead_link check, etc.
            }
        }
        // Possibly build ack packets
        // In real KCP, you'd track pending ack and flush them too.
        // For minimal example, we skip that or do it inline in parse_data.
    }

    fn parse_una(&mut self, una: u32) {
        debug!(
            "[ConvID: {}, Thread: {:?}] parse_una(una={})",
            self.conv,
            thread::current().id(),
            una
        );
        // Remove *all* segments in snd_buf where seg.sn < una
        // KCP's UNA confirms receipt of all segments *before* it.
        let original_len = self.snd_buf.len();
        {
            let pre_retain_sns: Vec<u32> = self.snd_buf.iter().map(|seg| seg.sn).collect();
            debug!(
                "[ConvID: {}, Thread: {:?}] parse_una: Pre-retain snd_buf (len={}): {:?}",
                self.conv,
                thread::current().id(),
                original_len,
                pre_retain_sns
            );
        }
        self.snd_buf.retain(|seg| seg.sn >= una);
        let removed_count = original_len.saturating_sub(self.snd_buf.len());

        // Log state *after* retain
        let post_retain_sns: Vec<u32> = self.snd_buf.iter().map(|seg| seg.sn).collect();
        debug!(
            "[ConvID: {}, Thread: {:?}] parse_una: Post-retain snd_buf (len={}): {:?}",
            self.conv,
            thread::current().id(),
            self.snd_buf.len(),
            post_retain_sns
        );
        // Corrected format string arguments for the removed count log
        debug!("[ConvID: {}, Thread: {:?}] parse_una(una={}): Removed {} segment(s) from snd_buf ({} -> {}). Remaining sns: {:?}",
                self.conv, thread::current().id(), una, removed_count, original_len, self.snd_buf.len(), post_retain_sns);

        if removed_count > 0 {
            // Use trace level if no segments were removed but buffer wasn't empty
            debug!(
                "[ConvID: {}, Thread: {:?}] parse_una(una={}): No segments removed from snd_buf (len={}). Remaining sns: {:?}",
                self.conv,
                thread::current().id(),
                una,
                original_len,
                self.snd_buf.iter().map(|s| s.sn).collect::<Vec<_>>()
            );
        }

        // Update the known acknowledged sequence number.
        // Use max to prevent out-of-order packets with older UNA from moving snd_una backwards.
        self.snd_una = cmp::max(self.snd_una, una);
    }

    fn parse_ack(&mut self, sn: u32, ts: u32) {
        debug!(
            "[ConvID: {}, Thread: {:?}] Parsing ack, sn: {}, ts: {}",
            self.conv,
            thread::current().id(),
            sn,
            ts
        );
        // find the segment in snd_buf
        if let Some(pos) = self.snd_buf.iter().position(|seg| seg.sn == sn) {
            let seg = self.snd_buf.remove(pos).unwrap();
            debug!(
                "[ConvID: {}, Thread: {:?}] Acked segment, sn: {}, frg: {}",
                self.conv,
                thread::current().id(),
                sn,
                seg.frg
            );
            // update RTT
            let rtt = self.current.saturating_sub(ts);
            self.update_rtt(rtt);
        } else {
            // Log if the segment was NOT found
            let current_sns: Vec<u32> = self.snd_buf.iter().map(|s| s.sn).collect();
            warn!(
                "[ConvID: {}, Thread: {:?}] parse_ack: ACK received for sn={}, but segment not found in snd_buf (len={}): {:?}",
                self.conv,
                thread::current().id(),
                sn,
                self.snd_buf.len(),
                current_sns
            );
        }
    }

    fn parse_data(&mut self, pkt: &KcpPacket) {
        // Insert into rcv_buf if pkt.sn in [rcv_nxt .. rcv_nxt + rcv_wnd)
        if pkt.sn() >= self.rcv_nxt + self.rcv_wnd as u32 {
            // out of window
            return;
        }
        if pkt.sn() < self.rcv_nxt {
            // already got it, discard
            return;
        }

        // Check if we have it
        let mut insert_idx = self.rcv_buf.len();
        for (i, seg) in self.rcv_buf.iter().enumerate() {
            #[allow(clippy::comparison_chain)]
            if pkt.sn() < seg.sn {
                insert_idx = i;
                break;
            } else if pkt.sn() == seg.sn {
                // duplicate
                return;
            }
        }

        let seg = Segment {
            sn: pkt.sn(),
            frg: pkt.frg(),
            ts: pkt.ts(),
            resendts: 0,
            rto: 0,
            xmit: 0,
            data: pkt.data().into(),
        };
        self.rcv_buf.insert(insert_idx, seg);

        // Move ready segments from rcv_buf -> rcv_queue
        self.move_buf_to_queue();
    }

    fn move_buf_to_queue(&mut self) {
        // Loop as long as we can potentially extract a complete message from the front
        loop {
            // Check if the buffer starts with the next expected sequence number
            if self.rcv_buf.is_empty() || self.rcv_buf[0].sn != self.rcv_nxt {
                break; // Cannot start assembling a message now
            }

            // Scan ahead in rcv_buf to find if a complete message exists contiguously
            let mut end_segment_index = None;
            let mut expected_sn = self.rcv_nxt;
            let mut message_data_len = 0;

            for (idx, seg) in self.rcv_buf.iter().enumerate() {
                if seg.sn != expected_sn {
                    // Found a gap before completing a message
                    end_segment_index = None;
                    break;
                }
                message_data_len += seg.data.len();
                if seg.frg == 0 {
                    // Found the last fragment of a message
                    end_segment_index = Some(idx);
                    break;
                }
                expected_sn = expected_sn.wrapping_add(1);
            }

            // If we didn't find a complete message sequence at the front
            if end_segment_index.is_none() {
                break;
            }

            let end_idx = end_segment_index.unwrap();

            // We found a complete message spanning indices 0..=end_idx
            // Assemble it and move to rcv_queue
            let mut message_buf = BytesMut::with_capacity(message_data_len);
            let mut final_sn = 0;
            for _ in 0..=end_idx {
                // pop_front is efficient for VecDeque
                let seg = self.rcv_buf.pop_front().unwrap();
                message_buf.extend_from_slice(&seg.data);
                final_sn = seg.sn;
            }

            // Push the fully assembled message
            self.rcv_queue.push_back(message_buf);

            // Update the next expected sequence number
            self.rcv_nxt = final_sn.wrapping_add(1);

            // Loop again to see if the *next* message is also ready
        }
    }

    fn ack_push(&mut self, sn: u32, ts: u32) {
        debug!("Acking, sn: {}, ts: {}", sn, ts);
        let pkt = KcpPacket::new(
            self.conv,
            KcpCommand::Ack,
            0,
            self.rcv_wnd,
            ts,
            sn,
            self.rcv_nxt, // next expected
            Vec::new(),
        );
        self.out_pkts.push(pkt);
    }

    fn update_rtt(&mut self, rtt: u32) {
        if self.rx_srtt == 0 {
            self.rx_srtt = rtt;
            self.rx_rttval = rtt / 2;
        } else {
            let delta = rtt.abs_diff(self.rx_srtt);
            self.rx_rttval = (3 * self.rx_rttval + delta) / 4;
            self.rx_srtt = (7 * self.rx_srtt + rtt) / 8;
            if self.rx_srtt < 1 {
                self.rx_srtt = 1;
            }
        }
        let rto = self.rx_srtt + cmp::max(self.interval, 4 * self.rx_rttval);
        self.rx_rto = rto.clamp(self.rx_minrto, 60000);
    }
}

impl Read for KcpSession {
    /// Reads data from the KCP session into `buf`.
    ///
    /// If there's no data in `rcv_queue`, it returns `Ok(0)`,
    /// indicating no more data is currently available.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.recv(buf);
        // If `n == 0`, it means there's no data right now.
        // For a standard `Read` trait, returning `Ok(0)` indicates EOF or no data available.
        Ok(n)
    }
}

impl Write for KcpSession {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // If there's no data, trivially done
        if buf.is_empty() {
            return Ok(0);
        }

        // 1) How many segments can we add right now?
        let avail_segs = self.available_send_segments();
        if avail_segs == 0 {
            // We have no space to queue even a single segment.
            // Return a WouldBlock error so the caller knows they should retry later.
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Send window is full",
            ));
        }

        // 2) How many segments would be needed to store all of `buf`?
        // We have an `mtu` that we use in `send()` to break data up.
        let needed_segs = buf.len().div_ceil(self.mtu);

        // 3) How many segments can we actually accept?
        let accept_segs = needed_segs.min(avail_segs);

        // 4) If we accept N segments, that corresponds to `N * mtu` bytes (or the remainder if the buffer is smaller).
        let max_bytes = accept_segs * self.mtu;
        // But the buffer might be smaller than that, so clamp to `buf.len()`.
        let to_write = max_bytes.min(buf.len());

        // 5) If `to_write` is 0 but `avail_segs > 0`, that means
        //    the buffer is extremely small (less than 1?), or some edge case.
        //    Typically won't happen if `buf.len() > 0` and `avail_segs >= 1`.
        if to_write == 0 {
            return Ok(0);
        }

        // 6) Actually queue that many bytes.
        let data_slice = &buf[..to_write];
        self.send(data_slice);

        // 7) Return how many bytes we queued
        Ok(to_write)
    }

    fn flush(&mut self) -> io::Result<()> {
        // KCP handles flush in `update()`, so no-op or
        // force a flush if you want immediate
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{KcpCommand, KcpPacket};
    use bytes::{Bytes, BytesMut};
    use env_logger;
    use log::debug;
    use std::io::Write;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_out_of_order_delivery_completes_correctly() {
        let conv_id = 12345;
        let mut sender = KcpSession::new(conv_id);
        let mut receiver = KcpSession::new(conv_id);

        // Set small MTU to force fragmentation
        let mtu = 20; // Small enough to split our message
        sender.set_mtu(mtu);

        // Message that will be fragmented
        let message = b"This message requires multiple KCP segments";
        let message_len = message.len();

        // Send the message
        sender.send(message);

        // Trigger update to move segments to snd_buf and create packets
        // Use the session's interval to ensure ts_flush is met
        sender.update(sender.interval);
        let packets = sender.fetch_outgoing();
        assert!(packets.len() > 1, "Message should have been fragmented");

        // Simulate out-of-order delivery: Deliver first and last packets only
        let first_packet = packets[0].clone();
        let last_packet = packets.last().unwrap().clone();

        println!(
            "Receiver state before any input: rcv_nxt={}, rcv_buf_len={}, rcv_queue_len={}",
            receiver.rcv_nxt,
            receiver.rcv_buf.len(),
            receiver.rcv_queue.len()
        );

        println!("Inputting first packet (sn={})", first_packet.sn());
        receiver.input(&first_packet);
        receiver.update(0); // Process input
        println!(
            "Receiver state after first packet: rcv_nxt={}, rcv_buf_len={}, rcv_queue_len={}",
            receiver.rcv_nxt,
            receiver.rcv_buf.len(),
            receiver.rcv_queue.len()
        );

        // The original bug would potentially push the first fragment here.
        // We assert that no complete message is available yet.
        let mut recv_buffer = BytesMut::with_capacity(message_len + 100);
        recv_buffer.resize(message_len + 100, 0); // Initialize buffer
        let bytes_read_partial = receiver.recv(recv_buffer.as_mut());
        assert_eq!(
            bytes_read_partial, 0,
            "Receiver should not have data yet (only first fragment received)"
        );
        assert!(
            receiver.rcv_queue.is_empty(),
            "Receive queue should be empty"
        );

        println!("Inputting last packet (sn={})", last_packet.sn());
        receiver.input(&last_packet);
        receiver.update(0); // Process input
        println!(
            "Receiver state after last packet: rcv_nxt={}, rcv_buf_len={}, rcv_queue_len={}",
            receiver.rcv_nxt,
            receiver.rcv_buf.len(),
            receiver.rcv_queue.len()
        );

        // Still no complete message should be available
        let bytes_read_partial2 = receiver.recv(recv_buffer.as_mut());
        assert_eq!(
            bytes_read_partial2, 0,
            "Receiver should not have data yet (first and last fragments received, middle missing)"
        );
        assert!(
            receiver.rcv_queue.is_empty(),
            "Receive queue should still be empty"
        );

        // Now, deliver the missing middle packets
        let middle_packets = packets[1..packets.len() - 1].to_vec();
        if !middle_packets.is_empty() {
            println!(
                "Inputting middle packets (sn={:?})",
                middle_packets.iter().map(|p| p.sn()).collect::<Vec<_>>()
            );
            for pkt in middle_packets {
                receiver.input(&pkt);
            }
            receiver.update(0); // Process input
        }
        println!(
            "Receiver state after middle packets: rcv_nxt={}, rcv_buf_len={}, rcv_queue_len={}",
            receiver.rcv_nxt,
            receiver.rcv_buf.len(),
            receiver.rcv_queue.len()
        );

        // NOW the complete message should be available
        let bytes_read_final = receiver.recv(recv_buffer.as_mut());
        assert_eq!(
            bytes_read_final, message_len,
            "Receiver should have the complete message now"
        );
        assert_eq!(
            &recv_buffer[..bytes_read_final],
            message,
            "Received message does not match sent message"
        );

        // Check if queue is empty after reading
        assert!(
            receiver.rcv_queue.is_empty(),
            "Receive queue should be empty after reading the message"
        );

        // Verify no more data
        let bytes_read_after = receiver.recv(recv_buffer.as_mut());
        assert_eq!(
            bytes_read_after, 0,
            "Receiver should have no more data after reading the message"
        );
    }

    #[test]
    fn test_congestion_window_limits_send_buffer() {
        init_logger();
        let conv = 123;
        let mut session = KcpSession::new(conv);
        session.set_mtu(50);

        session.snd_wnd = 10;
        session.rmt_wnd = 5;
        let initial_cwnd = std::cmp::min(session.snd_wnd, session.rmt_wnd);
        debug!(
            "Initial state: snd_wnd={}, rmt_wnd={}, calculated cwnd={}",
            session.snd_wnd, session.rmt_wnd, initial_cwnd
        );

        let data = Bytes::from(vec![1u8; 400]);
        session.send(&data);

        assert_eq!(
            session.snd_queue.len(),
            8,
            "Should have 8 segments in queue initially"
        );
        assert_eq!(
            session.snd_buf.len(),
            0,
            "Send buffer should be empty initially"
        );

        // Call update to move segments based on initial cwnd - *Use non-zero time*
        session.update(session.interval); // Use interval to trigger flush
        debug!(
            "After update 1: snd_buf_len={}, snd_queue_len={}",
            session.snd_buf.len(),
            session.snd_queue.len()
        );

        assert_eq!(
            session.snd_buf.len(),
            initial_cwnd as usize,
            "Send buffer should be limited by initial cwnd (5)"
        );
        assert_eq!(
            session.snd_queue.len(),
            8 - initial_cwnd as usize,
            "Queue should have remaining 3 segments"
        );

        let new_rmt_wnd = 8;
        let ack_packet = KcpPacket::new(
            conv,
            KcpCommand::Ack,
            0,
            new_rmt_wnd,
            0,
            0,
            session.rcv_nxt,
            Vec::new(),
        );
        session.input(&ack_packet);
        assert_eq!(
            session.rmt_wnd, new_rmt_wnd,
            "Remote window should be updated to 8"
        );

        let new_cwnd = std::cmp::min(session.snd_wnd, session.rmt_wnd);
        debug!(
            "After ACK: snd_wnd={}, rmt_wnd={}, calculated cwnd={}",
            session.snd_wnd, session.rmt_wnd, new_cwnd
        );

        // Call update again to move more segments based on the new cwnd - *Use non-zero time*
        session.update(session.interval); // Use interval to trigger flush
        debug!(
            "After update 2: snd_buf_len={}, snd_queue_len={}",
            session.snd_buf.len(),
            session.snd_queue.len()
        );

        // Check that snd_buf now contains segments up to the new cwnd (8)
        // The total number of segments should be 7 (initial 5 - 1 acked + 3 moved from queue)
        let expected_buf_len_after_ack = initial_cwnd as usize - 1 + (8 - initial_cwnd as usize);
        assert_eq!(
            session.snd_buf.len(),
            7,
            "Send buffer should contain 7 segments after acking sn=0 and refilling"
        );
        assert_eq!(
            session.snd_queue.len(),
            0,
            "Queue should be empty as all remaining segments were moved"
        );

        let mut session2 = KcpSession::new(conv + 1);
        session2.set_mtu(50);
        session2.snd_wnd = 3;
        session2.rmt_wnd = 10;
        let cwnd2 = std::cmp::min(session2.snd_wnd, session2.rmt_wnd);
        debug!(
            "Scenario 3: snd_wnd={}, rmt_wnd={}, calculated cwnd={}",
            session2.snd_wnd, session2.rmt_wnd, cwnd2
        );

        let data2 = Bytes::from(vec![5u8; 200]);
        session2.send(&data2);
        assert_eq!(
            session2.snd_queue.len(),
            4,
            "Session 2: Should have 4 segments in queue"
        );

        // Call update to move segments based on cwnd2 - *Use non-zero time*
        session2.update(session2.interval); // Use interval to trigger flush
        debug!(
            "Scenario 3 After update: snd_buf_len={}, snd_queue_len={}",
            session2.snd_buf.len(),
            session2.snd_queue.len()
        );

        assert_eq!(
            session2.snd_buf.len(),
            cwnd2 as usize,
            "Session 2: Send buffer should be limited by snd_wnd (3)"
        );
        assert_eq!(
            session2.snd_queue.len(),
            4 - cwnd2 as usize,
            "Session 2: Queue should have remaining 1 segment"
        );
    }

    #[test]
    fn test_segment_retransmission_after_rto() {
        init_logger();
        let conv = 456;
        let mut session = KcpSession::new(conv);
        session.set_mtu(50);

        let data = Bytes::from(vec![2u8; 30]); // Single segment
        session.send(&data);
        assert_eq!(session.snd_queue.len(), 1, "Should have 1 segment in queue");

        // Initial update moves to snd_buf and prepares the first packet
        session.update(session.interval);
        assert_eq!(session.snd_buf.len(), 1, "Segment should be in send buffer");
        assert_eq!(session.snd_queue.len(), 0, "Queue should be empty");

        // Check segment details
        let segment = session
            .snd_buf
            .front()
            .expect("Segment must be in buffer")
            .clone(); // Clone for inspection
        let initial_rto = session.rx_rto;
        let expected_resendts = session.current + initial_rto;
        assert_eq!(segment.xmit, 1, "Initial transmit count should be 1");
        assert_eq!(
            segment.rto, initial_rto,
            "Segment RTO should match session RTO"
        );
        // Note: The actual resendts is set *inside* flush_outgoing AFTER moving to buf.
        // We need to call fetch_outgoing to ensure flush_outgoing ran fully.

        debug!(
            "Initial state: current={}, interval={}, rto={}, segment_sn={}",
            session.current, session.interval, initial_rto, segment.sn
        );

        // Fetch and discard the first packet (simulate loss)
        let initial_packets = session.fetch_outgoing();
        assert_eq!(
            initial_packets.len(),
            1,
            "Should have fetched 1 packet initially"
        );
        assert_eq!(
            initial_packets[0].sn(),
            segment.sn,
            "Packet SN should match segment SN"
        );
        debug!("Simulated loss of packet with sn={}", segment.sn);

        // We need the exact resend timestamp set by flush_outgoing
        let segment_in_buf = session
            .snd_buf
            .front()
            .expect("Segment must still be in buffer");
        let actual_resendts = segment_in_buf.resendts;
        debug!("Segment resendts timestamp: {}", actual_resendts);
        assert!(
            actual_resendts > session.current,
            "Resend timestamp should be in the future"
        );

        // Advance time to just before the retransmission timestamp
        let time_to_advance_almost = actual_resendts
            .saturating_sub(session.current)
            .saturating_sub(1);
        if time_to_advance_almost > 0 {
            session.update(time_to_advance_almost);
            debug!(
                "Advanced time by {}, current is now {}",
                time_to_advance_almost, session.current
            );
            let packets_before_rto = session.fetch_outgoing();
            assert!(
                packets_before_rto.is_empty(),
                "Should not retransmit before RTO expires"
            );
        }

        // Advance time past the retransmission timestamp
        let time_to_advance_past_rto = session.interval; // Advance by interval to ensure flush happens
        session.update(time_to_advance_past_rto);
        debug!(
            "Advanced time by {}, current is now {}, should be >= {}",
            time_to_advance_past_rto, session.current, actual_resendts
        );
        assert!(
            session.current >= actual_resendts,
            "Current time should now be past resendts"
        );

        // Fetch outgoing packets - should contain the retransmission
        let retransmitted_packets = session.fetch_outgoing();
        assert_eq!(
            retransmitted_packets.len(),
            1,
            "Should have retransmitted 1 packet"
        );
        assert_eq!(
            retransmitted_packets[0].sn(),
            segment.sn,
            "Retransmitted packet SN should match original"
        );

        // Verify transmit count increased
        let segment_after_retransmit = session
            .snd_buf
            .front()
            .expect("Segment must still be in buffer after retransmit");
        assert_eq!(
            segment_after_retransmit.xmit, 2,
            "Transmit count (xmit) should be 2 after retransmission"
        );
        debug!(
            "Retransmission confirmed for sn={}, xmit={}",
            segment_after_retransmit.sn, segment_after_retransmit.xmit
        );
    }

    #[test]
    fn test_ack_removes_segment_from_send_buffer() {
        init_logger();
        let conv = 789;
        let mut session = KcpSession::new(conv);
        session.set_mtu(50);

        let data = Bytes::from(vec![3u8; 40]); // Single segment
        session.send(&data);
        assert_eq!(session.snd_queue.len(), 1, "Should have 1 segment in queue");

        // Update to move to snd_buf
        session.update(session.interval);
        assert_eq!(session.snd_buf.len(), 1, "Segment should be in send buffer");
        assert_eq!(session.snd_queue.len(), 0, "Queue should be empty");

        // Get segment details (sn and ts are needed for the ACK)
        // Need ts from *after* flush_outgoing has run, which happens in update/fetch
        let _initial_packet = session.fetch_outgoing(); // Clears out_pkts and ensures ts is set
        assert_eq!(_initial_packet.len(), 1, "Should have created one packet");
        let segment_in_buf = session
            .snd_buf
            .front()
            .expect("Segment should be in buffer");
        let sn_to_ack = segment_in_buf.sn;
        let ts_for_ack = segment_in_buf.ts; // Timestamp when segment was originally sent
        debug!(
            "Segment sn={} ts={} is in snd_buf. Simulating ACK.",
            sn_to_ack, ts_for_ack
        );

        // Create ACK packet
        let ack_packet = KcpPacket::new(
            conv,
            KcpCommand::Ack,
            0,               // frg (unused for ACK)
            session.rcv_wnd, // Sender's current rcv_wnd (doesn't matter much for this test)
            ts_for_ack,      // ts must match the segment's ts for RTT calculation
            sn_to_ack,       // sn being acknowledged
            session.rcv_nxt, // una (doesn't matter much for this test)
            Vec::new(),      // data (empty for ACK)
        );

        // Input the ACK
        session.input(&ack_packet);

        // Verify the segment was removed from snd_buf
        assert!(
            session.snd_buf.is_empty(),
            "snd_buf should be empty after ACK processing"
        );
        debug!("ACK processed successfully, snd_buf is empty.");
    }

    #[test]
    fn test_ack_updates_rtt() {
        init_logger();
        let conv = 101;
        let mut session = KcpSession::new(conv);
        session.set_mtu(50);

        let initial_rto = session.rx_rto;
        debug!("Initial RTO: {}", initial_rto);
        // Set rx_minrto low for this test to ensure the calculated RTO isn't clamped
        // back to the initial_rto if the defaults were high.
        session.rx_minrto = 100; // Ensure calculated RTO (likely ~150ms) is > minrto

        let data = Bytes::from(vec![4u8; 20]); // Single segment
        session.send(&data);

        // Update to move to snd_buf and prepare packet
        session.update(session.interval);
        assert_eq!(session.snd_buf.len(), 1, "Segment should be in send buffer");

        // Fetch packet to ensure ts is set correctly in the segment
        let _packet = session.fetch_outgoing();
        assert_eq!(_packet.len(), 1, "Should have one packet");
        let segment_in_buf = session
            .snd_buf
            .front()
            .expect("Segment should still be in buffer");
        let sn_to_ack = segment_in_buf.sn;
        let ts_for_ack = segment_in_buf.ts;

        // Simulate RTT by advancing time *before* receiving ACK
        let simulated_rtt = 50; // ms
        session.update(simulated_rtt);
        debug!(
            "Advanced time by {}ms, current is now {}",
            simulated_rtt, session.current
        );

        // Create ACK packet
        let ack_packet = KcpPacket::new(
            conv,
            KcpCommand::Ack,
            0, // frg
            session.rcv_wnd,
            ts_for_ack,      // Original timestamp from segment
            sn_to_ack,       // SN being acked
            session.rcv_nxt, // una
            Vec::new(),      // data
        );

        // Input the ACK - this triggers parse_ack -> update_rtt
        session.input(&ack_packet);

        // Verify RTO has changed
        let new_rto = session.rx_rto;
        debug!("New RTO after ACK: {}", new_rto);
        assert_ne!(
            new_rto, initial_rto,
            "RTO should have been updated after receiving ACK with valid RTT"
        );

        // Verify segment is removed (as in previous test)
        assert!(
            session.snd_buf.is_empty(),
            "Segment should be removed by ACK"
        );
    }

    #[test]
    fn test_una_clears_send_buffer() {
        init_logger();
        let conv = 202;
        let mut session = KcpSession::new(conv);
        session.set_mtu(50);

        // Send 5 segments (SN 0, 1, 2, 3, 4)
        session.send(&vec![1u8; 30]); // sn=0
        session.send(&vec![2u8; 30]); // sn=1
        session.send(&vec![3u8; 30]); // sn=2
        session.send(&vec![4u8; 30]); // sn=3
        session.send(&vec![5u8; 30]); // sn=4
        assert_eq!(session.snd_queue.len(), 5);

        // Move all to snd_buf
        session.update(session.interval);
        let _ = session.fetch_outgoing(); // Discard packets
        assert_eq!(
            session.snd_buf.len(),
            5,
            "Should have 5 segments in snd_buf"
        );
        assert_eq!(session.snd_queue.len(), 0);
        debug!(
            "snd_buf initial contents (SNs): {:?}",
            session.snd_buf.iter().map(|s| s.sn).collect::<Vec<_>>()
        );

        // Simulate receiving a packet with una=3 (acks SN 0, 1, 2)
        let packet_with_una3 = KcpPacket::new(
            conv,
            KcpCommand::Ack, // Command type doesn't matter for UNA processing
            0,               // frg
            session.rcv_wnd, // wnd
            0,               // ts (dummy)
            0,               // sn (dummy)
            3,               // una = 3
            Vec::new(),      // data
        );
        session.input(&packet_with_una3);

        // Verify segments < 3 are removed
        assert_eq!(
            session.snd_buf.len(),
            2,
            "snd_buf should have 2 segments left after una=3"
        );
        let remaining_sns: Vec<u32> = session.snd_buf.iter().map(|s| s.sn).collect();
        assert_eq!(
            remaining_sns,
            vec![3, 4],
            "Remaining segments should be SN 3 and 4"
        );
        debug!("snd_buf contents after una=3: {:?}", remaining_sns);

        // Simulate receiving another packet with una=5 (acks SN 3, 4)
        let packet_with_una5 = KcpPacket::new(
            conv,
            KcpCommand::Push, // Try a different command type
            0,                // frg
            session.rcv_wnd,  // wnd
            0,                // ts (dummy)
            10,               // sn (dummy data sn)
            5,                // una = 5
            vec![9u8; 10],    // dummy data
        );
        session.input(&packet_with_una5);

        // Verify all segments < 5 are removed (buffer should be empty)
        assert!(
            session.snd_buf.is_empty(),
            "snd_buf should be empty after una=5"
        );
        debug!("snd_buf is empty after una=5");
    }

    #[test]
    fn test_write_fills_send_queue_when_window_full() {
        init_logger();
        let mut session = KcpSession::new(456);
        session.set_mtu(100);
        // Set small windows => cwnd = 5
        session.snd_wnd = 5;
        session.rmt_wnd = 5;
        let cwnd = std::cmp::min(session.snd_wnd, session.rmt_wnd) as usize;

        let data = vec![0u8; 600]; // Enough for 6 segments
        let expected_bytes_written = cwnd * session.mtu; // write is limited by available_send_segments (based on snd_wnd)

        // Write the data - should accept only enough bytes for cwnd segments
        match session.write(&data) {
            Ok(n) => assert_eq!(
                n, expected_bytes_written,
                "Write should only accept {} bytes based on snd_wnd={}",
                expected_bytes_written, session.snd_wnd
            ),
            Err(e) => panic!("Write failed unexpectedly: {:?}", e),
        }

        // Check that only the accepted segments are initially in snd_queue
        let expected_segments_in_queue = expected_bytes_written / session.mtu;
        assert_eq!(
            session.snd_queue.len(),
            expected_segments_in_queue,
            "snd_queue should contain {} segments initially",
            expected_segments_in_queue
        );
        assert_eq!(
            session.snd_buf.len(),
            0,
            "snd_buf should be empty initially"
        );

        // Update the session - this triggers move_queue_to_buf
        session.update(session.interval);

        // Verify that all initially queued segments were moved to snd_buf (up to cwnd)
        assert_eq!(
            session.snd_buf.len(),
            cwnd,
            "snd_buf should contain cwnd ({}) segments after update",
            cwnd
        );
        assert_eq!(
            session.snd_queue.len(),
            0, // All initially accepted segments should have moved
            "snd_queue should be empty after update"
        );

        // Verify sequence numbers in snd_buf
        for i in 0..cwnd {
            assert_eq!(session.snd_buf[i].sn, i as u32);
        }
        // Since queue is empty, no need to check snd_queue[0].sn
        // assert_eq!(session.snd_queue[0].sn, cwnd as u32);
    }

    #[test]
    fn test_ack_prevents_retransmission() {
        init_logger();
        let conv = 303;
        let mut session = KcpSession::new(conv);
        session.set_mtu(50);
        session.set_interval(10); // Use a short interval for easier time management

        let data = vec![5u8; 30]; // Single segment
        session.send(&data);

        // Update to move to snd_buf and prepare first transmission
        // We need to advance time to at least ts_flush to trigger the move
        session.update(session.ts_flush());
        assert_eq!(session.snd_buf.len(), 1, "Segment should be in snd_buf");

        // Fetch the initial packet and get segment details
        let initial_packets = session.fetch_outgoing();
        assert_eq!(
            initial_packets.len(),
            1,
            "Should fetch one packet initially"
        );
        let segment_in_buf = session.snd_buf.front().expect("Segment must be in buffer");
        let sn_to_ack = segment_in_buf.sn;
        let ts_for_ack = segment_in_buf.ts;
        let original_resendts = segment_in_buf.resendts;
        debug!(
            "Sent segment sn={}, ts={}, initial resendts={}",
            sn_to_ack, ts_for_ack, original_resendts
        );

        // Ensure resendts is in the future relative to current time
        assert!(
            original_resendts > session.current,
            "Original resendts should be in the future"
        );

        // --- Simulate receiving ACK before RTO expires --- //

        // Advance time slightly, but not past resendts
        let time_to_advance = 10;
        session.update(time_to_advance);
        debug!(
            "Advanced time by {}, current={}. Still before resendts.",
            time_to_advance, session.current
        );
        assert!(
            session.current < original_resendts,
            "Should still be before original resendts"
        );

        // Create and input the ACK packet
        let ack_packet = KcpPacket::new(
            conv,
            KcpCommand::Ack,
            0, // frg
            session.rcv_wnd,
            ts_for_ack,      // Original ts
            sn_to_ack,       // SN being acked
            session.rcv_nxt, // una
            Vec::new(),
        );
        session.input(&ack_packet);

        // Verify the segment is now gone due to the ACK
        assert!(
            session.snd_buf.is_empty(),
            "Segment should be removed by the ACK"
        );
        debug!("Received ACK for sn={}, snd_buf is now empty.", sn_to_ack);

        // --- Advance time PAST the original retransmission time --- //
        let time_to_advance_past_rto = original_resendts - session.current + session.interval;
        session.update(time_to_advance_past_rto);
        debug!(
            "Advanced time by {}, current={}. Now past original resendts.",
            time_to_advance_past_rto, session.current
        );
        assert!(
            session.current >= original_resendts,
            "Current time should be past original resendts"
        );

        // --- Verify no retransmission packet was generated --- //
        let packets_after_rto = session.fetch_outgoing();
        assert!(
            packets_after_rto.is_empty(),
            "No packets should be generated, as the segment was ACKed before RTO"
        );
        debug!("Confirmed no retransmission occurred.");
    }

    #[test]
    fn test_duplicate_fragment_handling() {
        init_logger();
        let conv = 505;
        let mut sender = KcpSession::new(conv);
        let mut receiver = KcpSession::new(conv);

        let mtu = 30;
        sender.set_mtu(mtu);
        receiver.set_mtu(mtu); // Receiver MTU doesn't strictly matter for input, but good practice

        let message = b"This is a message that will be fragmented into several parts.";
        let message_len = message.len();

        // Send the message
        sender.send(message);
        sender.update(sender.ts_flush());
        let packets = sender.fetch_outgoing();
        assert!(packets.len() > 1, "Message should have been fragmented");
        debug!("Sent {} fragments for the message.", packets.len());

        // Simulate receiving all fragments correctly first
        debug!("Simulating initial reception of all fragments...");
        for pkt in &packets {
            receiver.input(pkt);
        }
        receiver.update(0); // Process inputs

        // Verify the message is assembled in the receive queue
        assert_eq!(
            receiver.rcv_queue.len(),
            1,
            "Receive queue should have 1 complete message"
        );
        assert_eq!(
            receiver.rcv_buf.len(),
            0,
            "Receive buffer should be empty after assembling message"
        );
        let assembled_len = receiver.rcv_queue.front().map_or(0, |m| m.len());
        assert_eq!(
            assembled_len, message_len,
            "Assembled message length should match original"
        );
        debug!("Message correctly assembled initially.");

        // --- Simulate receiving a duplicate fragment (e.g., the second fragment) --- //
        assert!(packets.len() >= 2, "Test requires at least 2 fragments");
        let duplicate_packet = packets[1].clone(); // Clone the second fragment
        debug!(
            "Simulating reception of duplicate fragment sn={}",
            duplicate_packet.sn()
        );

        // Ensure rcv_nxt has advanced past the duplicate packet's sn
        assert!(
            receiver.rcv_nxt > duplicate_packet.sn(),
            "rcv_nxt should be past the duplicate sn"
        );

        receiver.input(&duplicate_packet);
        receiver.update(0); // Process the duplicate input

        // --- Verify state after duplicate --- //
        // 1. The receive buffer should still be empty (duplicate should be detected and discarded)
        assert_eq!(
            receiver.rcv_buf.len(),
            0,
            "Receive buffer should remain empty after duplicate"
        );
        // 2. The receive queue should still contain only the original complete message
        assert_eq!(
            receiver.rcv_queue.len(),
            1,
            "Receive queue should still have only 1 complete message"
        );
        let assembled_len_after_duplicate = receiver.rcv_queue.front().map_or(0, |m| m.len());
        assert_eq!(
            assembled_len_after_duplicate, message_len,
            "Assembled message length should be unchanged"
        );
        debug!("Duplicate fragment correctly ignored.");

        // --- Verify reading the message works correctly --- //
        let mut read_buffer = vec![0u8; message_len + 10];
        let bytes_read = receiver.recv(&mut read_buffer);
        assert_eq!(
            bytes_read, message_len,
            "recv should return the full message length"
        );
        assert_eq!(
            &read_buffer[..bytes_read],
            message,
            "Received message content should match original"
        );
        assert!(
            receiver.rcv_queue.is_empty(),
            "Receive queue should be empty after reading"
        );
        debug!("Message read successfully after duplicate ignored.");

        // Verify no more data
        let bytes_read_again = receiver.recv(&mut read_buffer);
        assert_eq!(bytes_read_again, 0, "Subsequent recv should return 0 bytes");
    }

    #[test]
    fn test_fragment_loss_and_reassembly() {
        init_logger();
        let conv = 606;
        let mut sender = KcpSession::new(conv);
        let mut receiver = KcpSession::new(conv);

        let mtu = 40; // Reduced MTU to ensure >= 3 fragments for the message
        sender.set_mtu(mtu);
        sender.set_interval(10);
        receiver.set_mtu(mtu);
        receiver.set_interval(10);

        let message = b"Testing fragment loss requires a message split into at least three parts for clarity.";
        let message_len = message.len();

        // Send the message
        sender.send(message);
        sender.update(sender.ts_flush()); // Move to snd_buf, set initial rto/resendts
        let packets = sender.fetch_outgoing();
        assert!(
            packets.len() >= 3,
            "Message should fragment into at least 3 parts for this test"
        );
        let num_fragments = packets.len();
        debug!("Sent {} fragments for the message.", num_fragments);

        // --- Simulate losing the second fragment --- //
        let lost_packet_sn = packets[1].sn();
        debug!("Simulating loss of fragment sn={}", lost_packet_sn);

        // Deliver all packets *except* the lost one
        for i in 0..num_fragments {
            if i != 1 {
                receiver.input(&packets[i]);
            }
        }
        receiver.update(0); // Process inputs

        // Verify message is incomplete
        let mut read_buffer = vec![0u8; message_len + 10];
        let bytes_read = receiver.recv(&mut read_buffer);
        assert_eq!(
            bytes_read, 0,
            "recv should return 0 as message is incomplete"
        );
        assert!(
            !receiver.rcv_buf.is_empty(),
            "Receive buffer should contain the received fragments"
        );
        assert!(
            receiver.rcv_queue.is_empty(),
            "Receive queue should be empty"
        );
        debug!(
            "Receiver state after initial partial delivery: rcv_buf size {}, rcv_queue size {}",
            receiver.rcv_buf.len(),
            receiver.rcv_queue.len()
        );

        // --- Simulate ACKs for received packets (sn=0, sn=2) going back to sender --- //
        let receiver_acks = receiver.fetch_outgoing();
        debug!(
            "Receiver generated {} ACK packets for received fragments.",
            receiver_acks.len()
        );
        for ack_pkt in receiver_acks {
            // Ensure these are ACKs and have relevant SNs if needed for debugging
            assert_eq!(
                ack_pkt.cmd(),
                KcpCommand::Ack,
                "Packet from receiver should be an ACK"
            );
            debug!(
                "Sender processing ACK for sn={}, ts={}",
                ack_pkt.sn(),
                ack_pkt.ts()
            );
            sender.input(&ack_pkt);
        }
        // After processing ACKs, sn=0 and sn=2 should be removed from sender's snd_buf
        assert_eq!(
            sender.snd_buf.len(),
            1,
            "Sender snd_buf should only contain the unacked lost segment (sn=1)"
        );
        assert_eq!(
            sender.snd_buf[0].sn, lost_packet_sn,
            "Remaining segment in sender snd_buf should be the lost one"
        );

        // --- Trigger retransmission on sender --- //

        // Find the segment corresponding to the lost packet in sender's buffer
        let lost_segment = sender
            .snd_buf
            .iter()
            .find(|seg| seg.sn == lost_packet_sn)
            .expect("Lost segment must be in sender's snd_buf");
        let original_resendts = lost_segment.resendts;
        let current_sender_time = sender.ts_current();
        debug!(
            "Lost segment sn={} has original resendts={}, current sender time={}",
            lost_packet_sn, original_resendts, current_sender_time
        );
        assert!(
            original_resendts > current_sender_time,
            "resendts should be in the future"
        );

        // Advance time past the RTO
        let time_to_advance = original_resendts - current_sender_time + sender.interval;
        sender.update(time_to_advance);
        debug!(
            "Advanced sender time by {}, current={}. Now past original resendts.",
            time_to_advance,
            sender.ts_current()
        );

        // Fetch the retransmitted packet
        let retransmit_packets = sender.fetch_outgoing();
        assert_eq!(
            retransmit_packets.len(),
            1,
            "Should have retransmitted exactly one packet"
        );
        let retransmitted_packet = &retransmit_packets[0];
        assert_eq!(
            retransmitted_packet.sn(),
            lost_packet_sn,
            "Retransmitted packet SN should match lost packet SN"
        );
        assert_eq!(
            retransmitted_packet.frg(),
            packets[1].frg(),
            "Retransmitted packet FRG should match lost packet FRG"
        );
        debug!(
            "Successfully fetched retransmitted packet sn={}",
            retransmitted_packet.sn()
        );

        // --- Deliver retransmitted packet and verify reassembly --- //
        receiver.input(retransmitted_packet);
        receiver.update(0); // Process the retransmitted packet

        // Verify message is now complete
        assert!(
            receiver.rcv_buf.is_empty(),
            "Receive buffer should be empty after receiving the missing fragment"
        );
        assert_eq!(
            receiver.rcv_queue.len(),
            1,
            "Receive queue should now contain the complete message"
        );
        let assembled_len = receiver.rcv_queue.front().map_or(0, |m| m.len());
        assert_eq!(
            assembled_len, message_len,
            "Assembled message length should match original"
        );
        debug!("Message reassembled successfully after retransmission.");

        // Read the message
        let bytes_read_final = receiver.recv(&mut read_buffer);
        assert_eq!(
            bytes_read_final, message_len,
            "recv should return the full message length after reassembly"
        );
        assert_eq!(
            &read_buffer[..bytes_read_final],
            message,
            "Received message content should match original"
        );
        assert!(
            receiver.rcv_queue.is_empty(),
            "Receive queue should be empty after reading"
        );

        // Verify no more data
        let bytes_read_again = receiver.recv(&mut read_buffer);
        assert_eq!(bytes_read_again, 0, "Subsequent recv should return 0 bytes");
    }
}
