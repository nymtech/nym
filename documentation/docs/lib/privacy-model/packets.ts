// Packet-framing model (pure, testable). Shows how an application message stream
// is chunked and framed for each transport:
//   - Mixnet: fixed-size 2413-byte Sphinx packets (348 B per-hop routing header
//     + 17 B payload overhead + 2048 B plaintext payload). Mixnet mode tunnels
//     IP, so the plaintext holds a 7 B fragmentation header, the IP-packet-router
//     framing, an IP packet, and padding. The IPR caps its IP bundle at 1500 B
//     (not the full 2048) to leave room for the SURB/MixAck the Sphinx packet
//     also carries, so ~548 B of the plaintext is reserved-plus-padding on every
//     packet. Each Sphinx packet is wrapped in a WebSocket binary frame and the
//     WebSocket stream is sent over TCP/IP. The reliable-channel return route
//     carries 56-byte mixACKs (NOT Sphinx packets) over TCP/IP in the same
//     WebSocket channel. Sizes trace to common/nymsphinx and
//     common/ip-packet-requests (sphinx-packet 0.6.0).
//   - dVPN: WireGuard encapsulation, two WireGuard headers (2-hop nested
//     tunnel) plus the remaining payload, up to a 1500-byte MTU (UDP). NOT
//     padded, so packet size varies.
//
// Ported from the source visualisations lib and generalised: the sample
// payloads are ordinary application messages (chat, DNS, API, web), not any one
// application's calls. Sizes are illustrative, chosen so the fragmentation and
// goodput story reads clearly, not measured.

export const MTU = 1500;
export const TCPIP_HEADER = 40; // IPv4 (20) + TCP (20)
export const MSS = MTU - TCPIP_HEADER; // 1460 bytes of payload per TCP segment

// Mixnet / Sphinx. Constants trace to common/nymsphinx (sphinx-packet =0.6.0):
// REGULAR_PACKET_SIZE = 2*1024 + HEADER_SIZE(348) + PAYLOAD_OVERHEAD_SIZE(17).
export const SPHINX_HEADER = 348; // sphinx-packet header::HEADER_SIZE
export const SPHINX_PAYLOAD_OVERHEAD = 17; // sphinx-packet PAYLOAD_OVERHEAD_SIZE
export const SPHINX_PLAINTEXT = 2 * 1024; // plaintext payload capacity = 2048
export const SPHINX_SIZE = SPHINX_HEADER + SPHINX_PAYLOAD_OVERHEAD + SPHINX_PLAINTEXT; // 2413
// Fragmentation header inside the plaintext (nymsphinx chunking,
// UNLINKED_FRAGMENTED_HEADER_LEN). Linked fragments use 10 B; we model the
// single/first-fragment case.
export const FRAG_HEADER = 7;
// IP-packet-router framing. Mixnet mode tunnels IP: each IP packet rides inside
// the Sphinx payload wrapped by the IPR. common/ip-packet-requests: a 2 B length
// prefix per IP packet (MultiIpPacketCodec) plus the bincode-varint
// IpPacketRequest wrapper (Protocol ~2 B + enum tag 1 B + length varint ~2 B).
export const IPR_LENGTH_PREFIX = 2;
export const IPR_WRAPPER = 5;
export const IPR_FRAMING = IPR_LENGTH_PREFIX + IPR_WRAPPER; // 7
// The IPR caps its bundle at MAX_PACKET_SIZE (1500), NOT the full 2048 plaintext,
// because the Sphinx packet also has to carry the SURB/MixAck ("can't just use
// 2kb", codec.rs). So usable IP-packet bytes per Sphinx packet is bounded here,
// and the headroom up to 2048 is reserved-plus-padding.
export const IPR_MAX_PACKET = 1500; // common/ip-packet-requests codec.rs MAX_PACKET_SIZE
export const SPHINX_CHUNK = IPR_MAX_PACKET - IPR_LENGTH_PREFIX; // usable app bytes = 1498
// WebSocket binary frame header carrying a 2413 B Sphinx packet, masked
// (client to server): 2 base + 2 (16-bit ext length, 126<len<=65535) + 4 mask = 8.
export const WS_HEADER = 8;
// Small unmasked return frame (server to client) for a 56 B mixACK: 2 base bytes.
export const WS_HEADER_ACK = 2;
// Reliable-channel ACK: 56 bytes, sent over TCP/IP in the WebSocket channel
// (NOT a full Sphinx packet).
export const MIXACK_SIZE = 56;

// dVPN / WireGuard (2-hop, two nested headers)
export const WG_HEADER = 60; // per-hop: WG transport (32) + outer IP/UDP (28)
export const WG_HEADERS_2HOP = WG_HEADER * 2; // 120
export const WG_PAYLOAD = MTU - WG_HEADERS_2HOP; // usable app bytes = 1380

export interface Payload {
  id: string;
  label: string;
  bytes: number;
  note: string;
}

// Representative application messages (illustrative sizes). A Sphinx packet
// carries ~1498 usable IP-packet bytes, so the small payloads fit in a single
// packet while the larger ones fragment into many, which is where the metering
// and goodput cost shows up.
export const PAYLOADS: Payload[] = [
  {
    id: "chat",
    label: "Chat message",
    bytes: 120,
    note: "A short text message: one fixed-size Sphinx packet, mostly padding.",
  },
  {
    id: "dns",
    label: "DNS query",
    bytes: 60,
    note: "A name lookup: tiny, still one full 2413 B Sphinx packet on the wire.",
  },
  {
    id: "api",
    label: "API request/response",
    bytes: 3500,
    note: "A few KB of structured data: a handful of packets, the comfortable mixnet workload.",
  },
  {
    id: "web",
    label: "Web page / bulk fetch",
    bytes: 48000,
    note: "A full page or bulk download: many packets, so send metering dominates and it can take seconds.",
  },
];

export type SegmentKind =
  | "sphinx-header"
  | "frag"
  | "ipr"
  | "payload"
  | "pad"
  | "ws"
  | "wg-entry"
  | "wg-exit";

export interface PacketSegment {
  label: string;
  bytes: number;
  kind: SegmentKind;
}

export interface Packet {
  index: number;
  segments: PacketSegment[];
  /** Application payload bytes actually carried (excludes framing/pad). */
  used: number;
  /** Total on-wire size of this packet (incl. WebSocket framing for mixnet). */
  size: number;
}

function total(segments: PacketSegment[]): number {
  return segments.reduce((s, seg) => s + seg.bytes, 0);
}

/**
 * Chunk a byte stream into fixed-size 2413-byte Sphinx packets, each wrapped in
 * a WebSocket binary frame. (TCP/IP segmentation is accounted for at the stream
 * level in `mixnetWire`, not per packet.)
 */
export function chunkMixnet(totalBytes: number): Packet[] {
  const n = Math.max(1, Math.ceil(totalBytes / SPHINX_CHUNK));
  const packets: Packet[] = [];
  let remaining = totalBytes;
  for (let i = 0; i < n; i++) {
    const chunk = Math.min(SPHINX_CHUNK, remaining);
    remaining -= chunk;
    // The plaintext (2048) always fills: frag + IPR framing + IP chunk + the rest
    // is padding, which includes the ~548 B the IPR reserves for the SURB/MixAck.
    const pad = SPHINX_PLAINTEXT - FRAG_HEADER - IPR_FRAMING - chunk;
    const segments: PacketSegment[] = [
      // Header segment also carries the 17 B per-hop payload overhead, so the
      // packet sums to the real 2413 B on the wire.
      {
        label: "Sphinx header",
        bytes: SPHINX_HEADER + SPHINX_PAYLOAD_OVERHEAD,
        kind: "sphinx-header",
      },
      { label: "fragmentation header", bytes: FRAG_HEADER, kind: "frag" },
      { label: "IPR framing", bytes: IPR_FRAMING, kind: "ipr" },
      { label: "IP packet", bytes: chunk, kind: "payload" },
    ];
    if (pad > 0)
      segments.push({ label: "SURB/MixAck reserved + padding", bytes: pad, kind: "pad" });
    segments.push({ label: "WebSocket header", bytes: WS_HEADER, kind: "ws" });
    packets.push({ index: i, segments, used: chunk, size: total(segments) });
  }
  return packets;
}

/** Chunk a byte stream into WireGuard-encapsulated (variable-size) packets. */
export function chunkDvpn(totalBytes: number): Packet[] {
  const n = Math.max(1, Math.ceil(totalBytes / WG_PAYLOAD));
  const packets: Packet[] = [];
  let remaining = totalBytes;
  for (let i = 0; i < n; i++) {
    const chunk = Math.min(WG_PAYLOAD, remaining);
    remaining -= chunk;
    const segments: PacketSegment[] = [
      { label: "WireGuard hdr (entry hop)", bytes: WG_HEADER, kind: "wg-entry" },
      { label: "WireGuard hdr (exit hop)", bytes: WG_HEADER, kind: "wg-exit" },
      { label: "payload", bytes: chunk, kind: "payload" },
    ];
    packets.push({ index: i, segments, used: chunk, size: total(segments) });
  }
  return packets;
}

export interface FramingSummary {
  packets: Packet[];
  packetCount: number;
  onWireBytes: number;
  overheadBytes: number;
  overheadPct: number;
}

/** Generic on-wire summary from a set of packets (used for dVPN). */
export function summarize(packets: Packet[], appBytes: number): FramingSummary {
  const onWireBytes = packets.reduce((s, p) => s + p.size, 0);
  const overheadBytes = onWireBytes - appBytes;
  return {
    packets,
    packetCount: packets.length,
    onWireBytes,
    overheadBytes,
    overheadPct: appBytes > 0 ? (overheadBytes / appBytes) * 100 : 0,
  };
}

export interface MixnetWire {
  packetCount: number;
  /** Sphinx packets + WebSocket headers + TCP/IP, forward. */
  forwardOnWire: number;
  /** mixACKs + WebSocket headers + TCP/IP, return route. */
  ackCount: number;
  ackOnWire: number;
  totalOnWire: number;
  goodputPct: number;
}

/**
 * Full mixnet on-wire cost for an application message, including the WebSocket
 * framing, TCP/IP segmentation, and the 56-byte mixACK reliable-channel return
 * route. Goodput = application bytes / total on-wire bytes.
 */
export function mixnetWire(appBytes: number): MixnetWire {
  const packetCount = Math.max(1, Math.ceil(appBytes / SPHINX_CHUNK));

  const forwardStream = packetCount * (SPHINX_SIZE + WS_HEADER);
  const forwardTcp = Math.ceil(forwardStream / MSS) * TCPIP_HEADER;
  const forwardOnWire = forwardStream + forwardTcp;

  // One mixACK per Sphinx packet on the reliable channel.
  const ackCount = packetCount;
  const ackStream = ackCount * (MIXACK_SIZE + WS_HEADER_ACK);
  const ackTcp = Math.ceil(ackStream / MSS) * TCPIP_HEADER;
  const ackOnWire = ackStream + ackTcp;

  const totalOnWire = forwardOnWire + ackOnWire;
  return {
    packetCount,
    forwardOnWire,
    ackCount,
    ackOnWire,
    totalOnWire,
    goodputPct: totalOnWire > 0 ? (appBytes / totalOnWire) * 100 : 0,
  };
}
