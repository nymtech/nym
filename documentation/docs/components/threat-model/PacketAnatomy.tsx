"use client";

import { useState } from "react";
import {
  chunkDvpn,
  chunkMixnet,
  PAYLOADS,
  mixnetWire,
  SPHINX_SIZE,
  summarize,
  WS_HEADER,
  MTU,
  type Packet,
  type SegmentKind,
} from "../../lib/privacy-model/packets";
import {
  computePathLatency,
  fragmentationMs,
  formatLatency,
  DEFAULT_LATENCY_PARAMS,
  type LatencyParams,
} from "../../lib/privacy-model/latency";
import type { NodeKind } from "../../lib/privacy-model/types";

// Colours for the transit-time breakdown (match the timing-bar palette).
const TIME_COLOR = {
  sol: "var(--mode-dvpn)",
  mix: "var(--mode-mixnet)",
  ack: "var(--nym-warn)",
  metering: "var(--node-sp)",
} as const;

const DEFAULT_MIX_STAGES: NodeKind[] = [
  "client",
  "entry",
  "mix",
  "mix",
  "mix",
  "exit",
  "destination",
];

interface PacketTiming {
  sol: number;
  mix: number;
  ack: number;
  /** One packet's transit = sol + mix + ack. */
  perPacket: number;
  /** Send metering across all packets = (N-1) x interval. */
  metering: number;
  total: number;
}

const SEG_COLOR: Record<SegmentKind, string> = {
  "sphinx-header": "var(--mode-mixnet)",
  ipr: "var(--node-ipr)",
  payload: "var(--nym-accent)",
  pad: "var(--nym-text-faint)",
  ws: "var(--nym-warn)",
  "wg-entry": "var(--mode-dvpn)",
  "wg-exit": "var(--node-gateway)",
};

function fmtBytes(b: number): string {
  return b >= 1024 ? `${(b / 1024).toFixed(1)} KB` : `${b} B`;
}

export function PacketAnatomy({
  mode,
  params = DEFAULT_LATENCY_PARAMS,
  stages = DEFAULT_MIX_STAGES,
}: {
  mode: "mixnet" | "dvpn";
  params?: LatencyParams;
  stages?: NodeKind[];
}) {
  const [payloadId, setPayloadId] = useState("api");
  const payload = PAYLOADS.find((c) => c.id === payloadId) ?? PAYLOADS[0];
  const isMix = mode === "mixnet";

  const packets = isMix ? chunkMixnet(payload.bytes) : chunkDvpn(payload.bytes);
  const scale = isMix ? SPHINX_SIZE + WS_HEADER : MTU;

  // Per-packet transit time and the summed transfer time (mixnet only).
  // Reacts to the deep-dive's geo/mixing sliders via `params`.
  let timing: PacketTiming | null = null;
  if (isMix) {
    const l = computePathLatency(stages, "mixnet", params);
    const metering = fragmentationMs(packets.length);
    const perPacket = l.propagationMs + l.mixingMs + l.ackMs;
    timing = {
      sol: l.propagationMs,
      mix: l.mixingMs,
      ack: l.ackMs,
      perPacket,
      metering,
      total: perPacket + metering,
    };
  }

  // Mixnet on-wire includes WebSocket + TCP/IP + the mixACK return route.
  const wire = isMix ? mixnetWire(payload.bytes) : null;
  const sum = summarize(packets, payload.bytes);
  const onWire = wire ? wire.totalOnWire : sum.onWireBytes;
  const goodput = wire
    ? wire.goodputPct
    : (payload.bytes / sum.onWireBytes) * 100;

  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexDirection: "column", gap: 14 }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
        {PAYLOADS.map((c) => (
          <button
            key={c.id}
            className={`btn ${c.id === payloadId ? "primary" : ""}`}
            onClick={() => setPayloadId(c.id)}
          >
            {c.label} · {fmtBytes(c.bytes)}
          </button>
        ))}
      </div>

      <p className="disclaimer" style={{ margin: 0 }}>
        {payload.note}
      </p>

      <div className="pkt-arrow">
        {fmtBytes(payload.bytes)} chunked into {packets.length}{" "}
        {isMix ? "Sphinx" : "WireGuard"} packet{packets.length > 1 ? "s" : ""} ↓
      </div>

      {/* column header for the timing column */}
      {timing && (
        <div className="pkt-row" style={{ marginBottom: -4 }}>
          <div style={{ flex: 1 }} />
          <div className="pkt-timing pkt-timing-head">transit time</div>
        </div>
      )}

      {/* packets */}
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {packets.map((p) => (
          <PacketRow key={p.index} packet={p} scale={scale} timing={timing} />
        ))}
      </div>

      {/* transit-time totals (mixnet only) */}
      {timing && (
        <div className="pkt-timing-totals">
          <div className="pkt-timing-totals-title">
            Total time to transfer {fmtBytes(payload.bytes)} · {packets.length}{" "}
            packet{packets.length > 1 ? "s" : ""}
          </div>
          <TotalRow color={TIME_COLOR.sol} label="Speed of light" ms={timing.sol} />
          <TotalRow color={TIME_COLOR.mix} label="Mixing delays" ms={timing.mix} />
          <TotalRow color={TIME_COLOR.ack} label="Reliable-channel ACK" ms={timing.ack} />
          <TotalRow
            color={TIME_COLOR.metering}
            label={`Send metering (${packets.length}x chunks)`}
            ms={timing.metering}
          />
          <div className="pkt-timing-total-grand">
            <span>Total</span>
            <span>{formatLatency(timing.total)}</span>
          </div>
          <p className="disclaimer" style={{ margin: "4px 0 0" }}>
            Illustrative model, not measured. Each Sphinx packet takes the same
            transit; chunking a larger payload meters more packets onto the
            network, so a bulk fetch can take seconds.
          </p>
        </div>
      )}

      {/* summary */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
        <span className="badge accent">Goodput ≈ {Math.round(goodput)}%</span>
        <span className="badge">
          {packets.length} packet{packets.length > 1 ? "s" : ""}
        </span>
        <span className="badge">on-wire {fmtBytes(onWire)}</span>
        {wire && (
          <span className="badge">
            + {wire.ackCount} mixACK{wire.ackCount > 1 ? "s" : ""} ({fmtBytes(wire.ackOnWire)})
          </span>
        )}
        <span className="badge">
          {isMix
            ? "Every Sphinx packet is a constant 2000 B (constant size removes size-based correlation; timing is handled separately by Poisson sending)"
            : "Variable packet size: WireGuard preserves size and timing (leaks)"}
        </span>
      </div>

      {/* legend */}
      <div className="legend" style={{ fontSize: "0.74rem" }}>
        {isMix ? (
          <>
            <Swatch kind="sphinx-header" label="Sphinx header ~400 B" />
            <Swatch kind="ipr" label="IPR framing ~30 B" />
            <Swatch kind="payload" label="payload chunk (≤1570 B)" />
            <Swatch kind="pad" label="padding → 2000 B Sphinx" />
            <Swatch kind="ws" label="WebSocket header ~8 B" />
          </>
        ) : (
          <>
            <Swatch kind="wg-entry" label="WireGuard hdr, entry hop ~60 B" />
            <Swatch kind="wg-exit" label="WireGuard hdr, exit hop ~60 B" />
            <Swatch kind="payload" label="remaining payload (≤1380 B)" />
          </>
        )}
      </div>
      {wire && (
        <p className="disclaimer" style={{ margin: 0 }}>
          Sphinx packets ride WebSocket binary frames over TCP/IP; the reliable
          channel returns a 56 B mixACK per packet (over TCP/IP, not a Sphinx
          packet). Goodput counts the ACK return route.
        </p>
      )}
    </div>
  );
}

function PacketRow({
  packet,
  scale,
  timing,
}: {
  packet: Packet;
  scale: number;
  timing: PacketTiming | null;
}) {
  return (
    <div className="pkt-row">
      <div style={{ flex: 1, minWidth: 0 }}>
        <div className="pkt-caption">
          packet #{packet.index + 1} · {fmtBytes(packet.size)}
        </div>
        <div className="pkt-track">
          <div className="pkt-fill" style={{ width: `${(packet.size / scale) * 100}%` }}>
            {packet.segments.map((seg, i) => (
              <div
                key={i}
                className="pkt-seg"
                style={{
                  width: `${(seg.bytes / packet.size) * 100}%`,
                  background: SEG_COLOR[seg.kind],
                  color: seg.kind === "payload" || seg.kind === "pad" ? "#10201a" : "#0e1417",
                }}
                title={`${seg.label}: ${seg.bytes} B`}
              >
                {seg.bytes >= 150 ? `${seg.bytes}` : ""}
              </div>
            ))}
          </div>
        </div>
      </div>
      {timing && (
        <div
          className="pkt-timing"
          title={`speed of light ${Math.round(timing.sol)} ms · mixing ${Math.round(
            timing.mix,
          )} ms · ACK ${Math.round(timing.ack)} ms`}
        >
          <div className="pkt-timing-total">{formatLatency(timing.perPacket)}</div>
          <div className="pkt-timing-bar">
            <span style={{ flex: timing.sol, background: TIME_COLOR.sol }} />
            <span style={{ flex: timing.mix, background: TIME_COLOR.mix }} />
            <span style={{ flex: timing.ack, background: TIME_COLOR.ack }} />
          </div>
        </div>
      )}
    </div>
  );
}

function TotalRow({ color, label, ms }: { color: string; label: string; ms: number }) {
  return (
    <div className="pkt-timing-total-row">
      <span className="legend-swatch" style={{ background: color }} />
      <span className="pkt-timing-total-label">{label}</span>
      <span className="pkt-timing-total-val">{formatLatency(ms)}</span>
    </div>
  );
}

function Swatch({ kind, label }: { kind: SegmentKind; label: string }) {
  return (
    <span className="legend-item">
      <span className="legend-swatch" style={{ background: SEG_COLOR[kind] }} />
      {label}
    </span>
  );
}
