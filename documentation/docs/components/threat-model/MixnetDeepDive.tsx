"use client";

import { useId, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { NodeGlyph } from "./NodeGlyph";
import { PacketAnatomy } from "./PacketAnatomy";
import { LatencyControls } from "./LatencyControls";
import {
  computePathLatency,
  DEFAULT_LATENCY_PARAMS,
  formatLatency,
  type LatencyParams,
} from "../../lib/privacy-model/latency";
import type { NodeKind } from "../../lib/privacy-model/types";
import { useReducedMotion } from "../../lib/privacy-model/useReducedMotion";

type Mode = "mixnet" | "dvpn";
interface XY {
  x: number;
  y: number;
}

const WIDTH = 960;
const PAD_X = 60;

export function MixnetDeepDive() {
  const reduced = useReducedMotion();
  const nodesPerLayerId = useId();
  const [mode, setMode] = useState<Mode>("mixnet");
  const [nodesPerLayer, setNodesPerLayer] = useState(6);
  const [playing, setPlaying] = useState(true);
  const [params, setParams] = useState<LatencyParams>(DEFAULT_LATENCY_PARAMS);
  const [regen, setRegen] = useState(0);

  const height = Math.max(300, nodesPerLayer * 34 + 96);
  const midY = height / 2;

  // Column x-positions.
  const cols = useMemo(() => {
    if (mode === "dvpn") {
      return { client: PAD_X, entry: 280, exit: 660, dest: WIDTH - PAD_X };
    }
    return {
      client: PAD_X,
      entry: 190,
      l1: 350,
      l2: 500,
      l3: 650,
      exit: 800,
      dest: WIDTH - PAD_X,
    } as Record<string, number>;
  }, [mode]);

  // Mix-layer node coordinates.
  const layers: XY[][] = useMemo(() => {
    if (mode === "dvpn") return [];
    const xs = [cols.l1, cols.l2, cols.l3];
    return xs.map((x) => {
      const arr: XY[] = [];
      const gap = (height - 80) / Math.max(1, nodesPerLayer - 1);
      for (let i = 0; i < nodesPerLayer; i++) {
        const y =
          nodesPerLayer === 1 ? midY : 40 + i * gap;
        arr.push({ x, y });
      }
      return arr;
    });
  }, [mode, nodesPerLayer, height, midY, cols]);

  const singles = useMemo(
    () => ({
      client: { x: cols.client, y: midY } as XY,
      entry: { x: cols.entry, y: midY } as XY,
      exit: { x: cols.exit, y: midY } as XY,
      dest: { x: cols.dest, y: midY } as XY,
    }),
    [cols, midY],
  );

  // Representative latency for the current mode (drives packet speed + readout).
  const repStages: NodeKind[] =
    mode === "mixnet"
      ? ["client", "entry", "mix", "mix", "mix", "exit", "destination"]
      : ["client", "entry", "exit", "destination"];
  const lat = computePathLatency(repStages, mode, params);
  const durationSec = Math.min(9, Math.max(1.5, lat.totalMs / 120));

  // Build the packet pool (regenerates on mode / node-count / regen change).
  const packets = useMemo(() => {
    const REAL = 14;
    const COVER = mode === "mixnet" ? 8 : 0;
    const out: {
      id: string;
      waypoints: XY[];
      color: string;
      r: number;
      duration: number;
      delay: number;
      opacityKeys: number[];
    }[] = [];

    const pick = (arr: XY[]) => arr[Math.floor(Math.random() * arr.length)];

    for (let i = 0; i < REAL; i++) {
      const wp: XY[] =
        mode === "mixnet"
          ? [
              singles.client,
              singles.entry,
              pick(layers[0]),
              pick(layers[1]),
              pick(layers[2]),
              singles.exit,
              singles.dest,
            ]
          : [singles.client, singles.entry, singles.exit, singles.dest];
      out.push({
        id: `real-${i}`,
        waypoints: wp,
        color: "var(--nym-accent)",
        r: 4.5,
        duration: durationSec * (0.85 + Math.random() * 0.4),
        delay: Math.random() * durationSec,
        opacityKeys: [0, 1, 1, 1, 0],
      });
    }

    // Cover traffic: faint loops among mix nodes (mixnet only).
    for (let i = 0; i < COVER; i++) {
      const a = pick(layers[0]);
      const b = pick(layers[1]);
      const wp: XY[] = [singles.entry, a, b, a, singles.entry];
      out.push({
        id: `cover-${i}`,
        waypoints: wp,
        color: "var(--node-mix)",
        r: 3,
        duration: durationSec * (0.6 + Math.random() * 0.4),
        delay: Math.random() * durationSec,
        opacityKeys: [0, 0.5, 0.5, 0.5, 0],
      });
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, nodesPerLayer, regen, durationSec]);

  const allMixNodes = layers.flat();

  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexDirection: "column", gap: 16 }}
    >
      {/* Controls */}
      <div
        className="card"
        style={{ display: "flex", flexWrap: "wrap", gap: 20, alignItems: "center" }}
      >
        <div style={{ display: "flex", gap: 8 }}>
          <button
            className={`btn ${mode === "mixnet" ? "primary" : ""}`}
            onClick={() => setMode("mixnet")}
          >
            Mixnet
          </button>
          <button
            className={`btn ${mode === "dvpn" ? "primary" : ""}`}
            onClick={() => setMode("dvpn")}
          >
            dVPN contrast
          </button>
        </div>
        {!reduced && (
          <button className="btn" onClick={() => setPlaying((p) => !p)}>
            {playing ? "❚❚ Pause" : "▶ Play"}
          </button>
        )}
        <button className="btn" onClick={() => setRegen((r) => r + 1)}>
          ↻ Reshuffle
        </button>
        {mode === "mixnet" && (
          <div className="slider-row" style={{ minWidth: 200 }}>
            <label htmlFor={nodesPerLayerId}>
              <span>Mix nodes / layer</span>
              <span>{nodesPerLayer}</span>
            </label>
            <input
              id={nodesPerLayerId}
              type="range"
              min={3}
              max={12}
              step={1}
              value={nodesPerLayer}
              onChange={(e) => setNodesPerLayer(Number(e.target.value))}
            />
          </div>
        )}
        <div style={{ minWidth: 240, flex: 1 }}>
          <LatencyControls params={params} onChange={setParams} />
        </div>
      </div>

      {/* Latency readout */}
      <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
        <span className="badge accent">
          {mode === "mixnet" ? "Mixnet 5-hop" : "dVPN direct"} total ≈{" "}
          {formatLatency(lat.totalMs)}
        </span>
        <span className="badge">
          Propagation {formatLatency(lat.propagationMs)}
        </span>
        {lat.mixingMs > 0 && (
          <span className="badge">Mixing {formatLatency(lat.mixingMs)}</span>
        )}
        {lat.ackMs > 0 && (
          <span className="badge">
            ACK return {formatLatency(lat.ackMs)}
          </span>
        )}
        <span className="badge">
          {mode === "mixnet"
            ? "Packets reorder at each layer; cover traffic hides real packets"
            : "Straight-through: order and timing preserved, no cover"}
        </span>
      </div>

      {/* Per-hop delays */}
      <div>
        <p className="section-title">Delay at each hop</p>
        <div className="card">
          <HopDelays mode={mode} params={params} />
        </div>
      </div>

      {/* Canvas */}
      <div className="diagram-wrap">
        <svg
          viewBox={`0 0 ${WIDTH} ${height}`}
          role="img"
          aria-label={
            mode === "mixnet"
              ? "Animated diagram: packets take independent random routes through three mix layers between the entry and exit gateways, with cover traffic looping among mix nodes, so output order differs from input order"
              : "Animated diagram: packets pass straight through from client to destination via a dVPN entry and exit gateway, preserving order and timing"
          }
        >
          {/* links: entry to each L1, L(i) to L(i+1) fully connected, L3 to exit */}
          {mode === "mixnet" && (
            <g opacity={0.18} stroke="var(--mode-mixnet)" strokeWidth={1}>
              {layers[0]?.map((n, i) => (
                <line
                  key={`e${i}`}
                  x1={singles.entry.x}
                  y1={singles.entry.y}
                  x2={n.x}
                  y2={n.y}
                />
              ))}
              {layers[0]?.map((a, i) =>
                layers[1]?.map((b, j) => (
                  <line key={`a${i}-${j}`} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />
                )),
              )}
              {layers[1]?.map((a, i) =>
                layers[2]?.map((b, j) => (
                  <line key={`b${i}-${j}`} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />
                )),
              )}
              {layers[2]?.map((n, i) => (
                <line
                  key={`x${i}`}
                  x1={n.x}
                  y1={n.y}
                  x2={singles.exit.x}
                  y2={singles.exit.y}
                />
              ))}
            </g>
          )}
          {mode === "dvpn" && (
            <g opacity={0.5} stroke="var(--mode-dvpn)" strokeWidth={2} strokeDasharray="6 5">
              <line
                x1={singles.client.x}
                y1={midY}
                x2={singles.dest.x}
                y2={midY}
              />
            </g>
          )}

          {/* packets */}
          {playing &&
            !reduced &&
            packets.map((p) => (
              <motion.circle
                key={`${p.id}-${regen}-${mode}`}
                r={p.r}
                fill={p.color}
                initial={{ cx: p.waypoints[0].x, cy: p.waypoints[0].y, opacity: 0 }}
                animate={{
                  cx: p.waypoints.map((w) => w.x),
                  cy: p.waypoints.map((w) => w.y),
                  opacity: p.opacityKeys,
                }}
                transition={{
                  duration: p.duration,
                  ease: "linear",
                  repeat: Infinity,
                  delay: p.delay,
                  repeatDelay: 0.15,
                }}
              />
            ))}
          {/* nodes */}
          <NodeStack singles={singles} layers={layers} mode={mode} />
          {/* reduced-motion: static representative packets, drawn after the
              nodes and offset above them so they stay visible */}
          {reduced &&
            allMixNodes
              .filter((_, i) => i % 2 === 0)
              .map((n, i) => (
                <circle key={i} cx={n.x} cy={n.y - 13} r={3.5} fill="var(--nym-accent)" />
              ))}
        </svg>
      </div>

      {/* Packet framing */}
      <div>
        <p className="section-title">
          Packet framing: {mode === "mixnet" ? "Sphinx" : "WireGuard"}
        </p>
        <div className="card">
          <PacketAnatomy mode={mode} params={params} stages={repStages} />
        </div>
      </div>

      {reduced && (
        <p className="disclaimer">
          Reduced-motion mode: animation disabled. The topology and latency model
          are shown statically.
        </p>
      )}
      <p className="disclaimer">
        Illustrative model: latency figures are pedagogical, not measured. Each
        real packet takes an independent random route through the mix layers, so
        output order differs from input order (per-packet unlinkability).
      </p>
    </div>
  );
}

function HopDelays({
  mode,
  params,
}: {
  mode: Mode;
  params: LatencyParams;
}) {
  // Ordered stops; mix layers additionally add a mixing delay.
  const stops =
    mode === "mixnet"
      ? ["Client", "Entry GW", "Mix L1", "Mix L2", "Mix L3", "Exit GW", "Destination"]
      : ["Client", "Entry GW", "Exit GW", "Destination"];

  let cumulative = 0;
  const rows = stops.map((name, i) => {
    const isMix = name.startsWith("Mix");
    const propagation = i === 0 ? 0 : params.geoMsPerHop; // hop into this stop
    const mixing = isMix && mode === "mixnet" ? params.mixDelayMs : 0;
    cumulative += propagation + mixing;
    return { name, propagation, mixing, cumulative, isMix };
  });

  return (
    <div>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "stretch",
          gap: 6,
        }}
      >
        {rows.map((r, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 6 }}>
            {i > 0 && (
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: "0.68rem",
                  color: "var(--nym-text-faint)",
                  textAlign: "center",
                  lineHeight: 1.2,
                }}
              >
                <div>+{r.propagation} ms</div>
                <div>net →</div>
              </div>
            )}
            <div
              className="card"
              style={{
                padding: "8px 10px",
                borderColor: r.isMix ? "var(--mode-mixnet)" : "var(--nym-border)",
                minWidth: 84,
                textAlign: "center",
              }}
            >
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "0.76rem" }}>
                {r.name}
              </div>
              {r.mixing > 0 && (
                <div
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: "0.68rem",
                    color: "var(--mode-mixnet)",
                  }}
                >
                  mix +{r.mixing} ms
                </div>
              )}
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: "0.68rem",
                  color: "var(--nym-accent)",
                }}
              >
                Σ {formatLatency(r.cumulative)}
              </div>
            </div>
          </div>
        ))}
      </div>
      <p className="disclaimer" style={{ marginTop: 10 }}>
        {mode === "mixnet"
          ? "Each hop adds ~speed-of-light propagation; each of the three mix layers adds an independent random mixing delay (Loopix). Σ is the running total to that hop."
          : "dVPN adds only propagation delay per hop, no mixing delay, so timing passes through unchanged."}
      </p>
    </div>
  );
}

function label(kind: NodeKind, x: number, y: number, text: string) {
  return (
    <text className="node-label" x={x} y={y} textAnchor="middle">
      {text}
    </text>
  );
}

function NodeStack({
  singles,
  layers,
  mode,
}: {
  singles: { client: XY; entry: XY; exit: XY; dest: XY };
  layers: XY[][];
  mode: Mode;
}) {
  return (
    <g>
      <NodeGlyph kind="client" x={singles.client.x} y={singles.client.y} />
      {label(
        "client",
        singles.client.x,
        singles.client.y + 30,
        "Client",
      )}
      <NodeGlyph kind="entry" x={singles.entry.x} y={singles.entry.y} />
      {label("entry", singles.entry.x, singles.entry.y + 32, "Entry GW")}

      {mode === "mixnet" &&
        layers.map((layer, li) =>
          layer.map((n, i) => (
            <NodeGlyph key={`m${li}-${i}`} kind="mix" x={n.x} y={n.y} r={9} />
          )),
        )}
      {mode === "mixnet" &&
        layers.map((layer, li) =>
          layer.length ? (
            <text
              key={`ll-${li}`}
              className="node-label"
              x={layer[0].x}
              y={18}
              textAnchor="middle"
            >
              Layer {li + 1}
            </text>
          ) : null,
        )}

      <NodeGlyph kind="exit" x={singles.exit.x} y={singles.exit.y} />
      {label("exit", singles.exit.x, singles.exit.y + 32, "Exit GW")}
      <NodeGlyph kind="destination" x={singles.dest.x} y={singles.dest.y} />
      <circle
        className="observer-halo"
        cx={singles.dest.x}
        cy={singles.dest.y}
        r={20}
      />
      {label("destination", singles.dest.x, singles.dest.y + 32, "Destination")}
    </g>
  );
}
