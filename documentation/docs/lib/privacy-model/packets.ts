// Packet-framing model (pure, testable). Shows how an application message stream
// is chunked and framed for each transport:
//   - Mixnet: fixed-size 2000-byte Sphinx packets (~400 B header + 1600 B
//     payload = IPR framing + chunked payload + padding). Each Sphinx packet is
//     wrapped in a WebSocket binary frame and the WebSocket stream is sent over
//     TCP/IP. The reliable-channel return route carries 56-byte mixACKs (NOT
//     Sphinx packets) over TCP/IP in the same WebSocket channel.
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

// Mixnet / Sphinx
export const SPHINX_SIZE = 2000; // a Sphinx packet is a constant 2000 bytes
export const SPHINX_HEADER = 400; // routing header (per-hop onion layers)
export const SPHINX_PAYLOAD = SPHINX_SIZE - SPHINX_HEADER; // 1600
export const IPR_FRAMING = 30; // IPR framing inside the Sphinx payload
export const SPHINX_CHUNK = SPHINX_PAYLOAD - IPR_FRAMING; // usable app bytes = 1570
// WebSocket binary frame header carrying a 2000 B Sphinx packet, masked
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
// carries ~1570 usable bytes, so the small payloads fit in a single packet
// while the larger ones fragment into many, which is where the metering and
// goodput cost shows up.
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
    note: "A name lookup: tiny, still one full 2000 B Sphinx packet on the wire.",
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
 * Chunk a byte stream into fixed-size 2000-byte Sphinx packets, each wrapped in
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
    const pad = SPHINX_CHUNK - chunk;
    const segments: PacketSegment[] = [
      { label: "Sphinx header", bytes: SPHINX_HEADER, kind: "sphinx-header" },
      { label: "IPR framing", bytes: IPR_FRAMING, kind: "ipr" },
      { label: "payload chunk", bytes: chunk, kind: "payload" },
    ];
    if (pad > 0)
      segments.push({ label: "padding (fixed size)", bytes: pad, kind: "pad" });
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
