"use client";

import { useId, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { NodeGlyph } from "./NodeGlyph";
import { PacketAnatomy } from "./PacketAnatomy";
import { useReducedMotion } from "../../lib/privacy-model/useReducedMotion";

const WIDTH = 960;
const EXIT_IP = "198.51.100.42";
const NUM_ENTRIES = 5;
// Deterministic per-entry request endpoints (stable across SSR/client, no random
// in rendered text). Each entry gateway's clients request a different endpoint,
// so the destination sees a spread of requests it cannot bind to a client.
const ENTRY_REQUESTS = ["/feed", "/search", "/assets", "/status", "/upload"];

export function DvpnCoverTraffic() {
  const reduced = useReducedMotion();
  const clientsId = useId();
  const [clients, setClients] = useState(10);
  const [playing, setPlaying] = useState(true);

  const height = Math.max(400, clients * 22 + 110);
  const midY = height / 2;
  const cols = { client: 66, entry: 330, exit: 650, dest: WIDTH - 70 };

  const entryPts = useMemo(
    () =>
      Array.from({ length: NUM_ENTRIES }, (_, e) => ({
        x: cols.entry,
        y: 60 + (e * (height - 120)) / (NUM_ENTRIES - 1),
        req: ENTRY_REQUESTS[e],
      })),
    [height, cols.entry],
  );

  // Distribute clients equally across the 5 entry gateways (round-robin).
  const clientPts = useMemo(() => {
    const groups: number[][] = Array.from({ length: NUM_ENTRIES }, () => []);
    for (let i = 0; i < clients; i++) groups[i % NUM_ENTRIES].push(i);
    const pts: { x: number; y: number; entry: number; delay: number }[] = [];
    groups.forEach((idxs, e) => {
      const base = entryPts[e].y;
      idxs.forEach((_, j) => {
        const y = base + (j - (idxs.length - 1) / 2) * 22;
        pts.push({
          x: cols.client - (j % 2) * 20,
          y,
          entry: e,
          delay: Math.random() * 2.6,
        });
      });
    });
    return pts;
  }, [clients, entryPts, cols.client]);

  const exit = { x: cols.exit, y: midY };
  const dest = { x: cols.dest, y: midY };

  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexDirection: "column", gap: 16 }}
    >
      <div className="card" style={{ display: "flex", flexWrap: "wrap", gap: 20, alignItems: "center" }}>
        {!reduced && (
          <button className="btn" onClick={() => setPlaying((p) => !p)}>
            {playing ? "❚❚ Pause" : "▶ Play"}
          </button>
        )}
        <div className="slider-row" style={{ minWidth: 240 }}>
          <label htmlFor={clientsId}>
            <span>Clients (spread over {NUM_ENTRIES} entry gateways)</span>
            <span>{clients}</span>
          </label>
          <input
            id={clientsId}
            type="range"
            min={NUM_ENTRIES}
            max={20}
            step={1}
            value={clients}
            onChange={(e) => setClients(Number(e.target.value))}
          />
        </div>
        <span className="badge accent">{clients} clients share the exit</span>
      </div>

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
        <span className="badge">
          {NUM_ENTRIES} entry gateways → 1 exit gateway → destination
        </span>
        <span className="badge">
          Each client sends a request; the endpoint varies per entry gateway
        </span>
        <span className="badge">
          The destination sees every request from one exit IP, but cannot map it
          to a client
        </span>
      </div>

      <div className="diagram-wrap">
        <svg
          viewBox={`0 0 ${WIDTH} ${height}`}
          role="img"
          aria-label={`Diagram: ${clients} clients spread across ${NUM_ENTRIES} entry gateways, all routed through a single exit gateway to the destination, which sees every request arriving from one exit IP`}
        >
          {/* client to entry links */}
          <g opacity={0.22} stroke="var(--mode-dvpn)" strokeWidth={1.1} strokeDasharray="5 4">
            {clientPts.map((c, i) => (
              <line key={i} x1={c.x} y1={c.y} x2={entryPts[c.entry].x} y2={entryPts[c.entry].y} />
            ))}
          </g>
          {/* entry to exit links */}
          <g stroke="var(--mode-dvpn)" strokeWidth={2} strokeDasharray="6 5" opacity={0.55}>
            {entryPts.map((en, e) => (
              <line key={e} x1={en.x} y1={en.y} x2={exit.x} y2={exit.y} />
            ))}
            <line x1={exit.x} y1={exit.y} x2={dest.x} y2={dest.y} />
          </g>

          {/* animated request packets: client to entry to exit to destination */}
          {playing &&
            !reduced &&
            clientPts.map((c, i) => {
              const en = entryPts[c.entry];
              const wp = [
                { x: c.x, y: c.y },
                { x: en.x, y: en.y },
                exit,
                dest,
              ];
              return (
                <motion.circle
                  key={`${i}-${clients}`}
                  r={4}
                  fill="var(--mode-dvpn)"
                  initial={{ cx: c.x, cy: c.y, opacity: 0 }}
                  animate={{
                    cx: wp.map((w) => w.x),
                    cy: wp.map((w) => w.y),
                    opacity: [0, 1, 1, 0],
                  }}
                  transition={{
                    duration: 2.6,
                    ease: "linear",
                    repeat: Infinity,
                    delay: c.delay,
                    repeatDelay: Math.random() * 0.6,
                  }}
                />
              );
            })}

          {/* clients */}
          {clientPts.map((c, i) => (
            <NodeGlyph key={i} kind="client" x={c.x} y={c.y} r={7} />
          ))}
          <text className="node-label" x={cols.client} y={16} textAnchor="middle">
            {clients} clients
          </text>

          {/* entry gateways with per-entry request endpoint */}
          {entryPts.map((en, e) => (
            <g key={e}>
              <NodeGlyph kind="entry" x={en.x} y={en.y} />
              <text className="node-label" x={en.x} y={en.y + 30} textAnchor="middle">
                Entry {e + 1}
              </text>
              <text
                x={en.x}
                y={en.y + 44}
                textAnchor="middle"
                style={{ fontFamily: "var(--font-mono)", fontSize: 9.5, fill: "var(--nym-accent)" }}
              >
                {en.req}
              </text>
            </g>
          ))}

          {/* exit + destination */}
          <NodeGlyph kind="exit" x={exit.x} y={exit.y} />
          <text className="node-label" x={exit.x} y={exit.y + 32} textAnchor="middle">
            Exit GW · {EXIT_IP}
          </text>
          <NodeGlyph kind="destination" x={dest.x} y={dest.y} />
          <circle className="observer-halo" cx={dest.x} cy={dest.y} r={20} />
          <text className="node-label" x={dest.x} y={dest.y + 32} textAnchor="middle">
            Destination
          </text>
        </svg>
      </div>

      <div>
        <p className="section-title">What the destination sees</p>
        <div className="card">
          <div className="obs-log">
            {Array.from({ length: Math.min(clients, 12) }).map((_, i) => (
              <div className="obs-row" key={i}>
                <span className="call">
                  GET {ENTRY_REQUESTS[i % NUM_ENTRIES]}
                </span>
                <span className="src">src={EXIT_IP} (exit GW) · client=?</span>
              </div>
            ))}
          </div>
          <p className="disclaimer" style={{ marginTop: 10 }}>
            Requests arrive from a single exit-gateway IP. The endpoint varies per
            entry gateway, and clients sharing an entry gateway send similar
            requests, so the destination sees a mix of requests it cannot bind to
            any client. With {clients} clients across {NUM_ENTRIES} gateways the
            destination-facing anonymity set is large. (dVPN hides the IP but not
            timing; pair with mixnet mode for timing safety.)
          </p>
        </div>
      </div>

      <div>
        <p className="section-title">dVPN packet framing</p>
        <div className="card">
          <PacketAnatomy mode="dvpn" />
        </div>
      </div>

      {reduced && (
        <p className="disclaimer">
          Reduced-motion mode: packet animation disabled; topology and observer
          log shown statically.
        </p>
      )}
    </div>
  );
}
