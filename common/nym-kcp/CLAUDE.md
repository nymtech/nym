# CLAUDE.md - nym-kcp

KCP (Fast and Reliable ARQ Protocol) implementation providing reliability over UDP for the Nym network. This crate ensures ordered, reliable delivery of packets.

## Architecture Overview

### Core Components

**KcpDriver** (src/driver.rs)
- High-level interface for KCP operations
- Manages single KCP session and I/O buffer
- Handles packet encoding/decoding

**KcpSession** (src/session.rs)
- Core KCP state machine
- Manages send/receive windows, RTT, congestion control
- Implements ARQ (Automatic Repeat Request) logic

**KcpPacket** (src/packet.rs)
- Wire format: conv(4B) | cmd(1B) | frg(1B) | wnd(2B) | ts(4B) | sn(4B) | una(4B) | len(4B) | data
- Commands: PSH (data), ACK, WND (window probe), ERR

## Key Concepts

### Conversation ID (conv)
- Unique identifier for each KCP connection
- Generated from hash of destination in nym-lp-node
- Must match on both ends for successful communication

### Packet Flow
1. **Send Path**: `send()` → Queue in send buffer → `fetch_outgoing()` → Wire
2. **Receive Path**: Wire → `input()` → Process ACKs/data → Application buffer
3. **Update Loop**: Call `update()` regularly to handle timeouts/retransmissions

### Reliability Mechanisms
- **Sequence Numbers (sn)**: Track packet ordering
- **Fragment Numbers (frg)**: Handle message fragmentation
- **UNA (Unacknowledged)**: Cumulative ACK up to this sequence
- **Selective ACK**: Via individual ACK packets
- **Fast Retransmit**: Triggered by duplicate ACKs
- **RTO Calculation**: Smoothed RTT with variance

## Configuration Parameters

```rust
// In KcpSession
MSS: 1400           // Maximum segment size
WINDOW_SIZE: 128    // Send/receive window
RTO_MIN: 100ms      // Minimum retransmission timeout
RTO_MAX: 60000ms    // Maximum retransmission timeout
FAST_RESEND: 2      // Fast retransmit threshold
```

## Common Operations

### Processing Incoming Data
```rust
driver.input(data)?;  // Decode and process packets
let packets = driver.fetch_outgoing();  // Get any response packets
```

### Sending Data
```rust
driver.send(&data);  // Queue for sending
driver.update(current_time);  // Trigger flush
let packets = driver.fetch_outgoing();  // Get packets to send
```

## Debugging Tips

- Enable `trace!` logs to see packet-level details
- Monitor `ts_flush` vs `ts_current` for timing issues
- Check `snd_wnd` and `rcv_wnd` for flow control problems
- Watch for "fast retransmit" messages indicating packet loss

## Integration Notes

- AIDEV-NOTE: MSS must account for Sphinx packet overhead
- AIDEV-NOTE: Window size affects memory usage and throughput
- Update frequency impacts latency vs CPU usage tradeoff
- Conv ID must be consistent across session lifecycle