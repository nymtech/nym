"use client";

import { useEffect, useId, useMemo, useRef, useState } from "react";
import { NodeGlyph } from "./NodeGlyph";
import { useReducedMotion } from "../../lib/privacy-model/useReducedMotion";

const WIDTH = 960;
const HEIGHT = 380;

const TICK_MS = 220;
const HISTORY = 52;

// Tor publishes on a rotation timer, so its upload rate is fixed by the clock
// and never by how many clients are reading the descriptor.
const TOR_ROTATION_TICKS = 34;

const CAPACITY = 24;
const LOW_WATER = 6;
const REFILL_BATCH = 18;
const ATTACKER_DRAIN = 2.2;

const CLIENT_COUNT = 3;

interface Tick {
  tor: boolean;
  nym: boolean;
}

export function SurbDirectory() {
  const reduced = useReducedMotion();
  const demandId = useId();

  const [demand, setDemand] = useState(8);
  const [playing, setPlaying] = useState(true);
  const [attacking, setAttacking] = useState(false);

  // One state object advanced by a pure updater. Splitting stock, history and
  // the clock across separate setters would need side effects inside an updater,
  // which StrictMode double-invokes and would double-count refills.
  const [sim, setSim] = useState({
    stock: CAPACITY,
    clock: 0,
    refills: 0,
    history: [] as Tick[],
  });

  // The simulation reads demand/attacking every tick, so hold them in a ref to
  // keep the interval stable rather than resubscribing on every slider move.
  const params = useRef({ demand, attacking });
  params.current = { demand, attacking };

  useEffect(() => {
    if (!playing || reduced) return;
    const id = setInterval(() => {
      setSim((prev) => {
        const { demand: d, attacking: atk } = params.current;
        const drained = prev.stock - (d * 0.12 + (atk ? ATTACKER_DRAIN : 0));
        const pushed = drained <= LOW_WATER;
        const clock = prev.clock + 1;
        const torPublished = clock % TOR_ROTATION_TICKS === 0;
        return {
          stock: pushed
            ? Math.min(CAPACITY, drained + REFILL_BATCH)
            : Math.max(0, drained),
          clock,
          refills: prev.refills + (pushed ? 1 : 0),
          history: [...prev.history, { tor: torPublished, nym: pushed }].slice(
            -HISTORY,
          ),
        };
      });
    }, TICK_MS);
    return () => clearInterval(id);
  }, [playing, reduced]);

  const { stock, history, refills } = sim;
  const torUploads = history.filter((t) => t.tor).length;
  const nymUploads = history.filter((t) => t.nym).length;

  const rows = { tor: 96, nym: 268 };
  const cols = { service: 90, dir: 430, client: 800 };

  const clientYs = useMemo(
    () =>
      Array.from(
        { length: CLIENT_COUNT },
        (_, i) => (i - (CLIENT_COUNT - 1) / 2) * 46,
      ),
    [],
  );

  const stockSlots = Math.round((stock / CAPACITY) * 8);

  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexDirection: "column", gap: 16 }}
    >
      <div
        className="card"
        style={{ display: "flex", flexWrap: "wrap", gap: 20, alignItems: "center" }}
      >
        {!reduced && (
          <button className="btn" onClick={() => setPlaying((p) => !p)}>
            {playing ? "❚❚ Pause" : "▶ Play"}
          </button>
        )}
        <div className="slider-row" style={{ minWidth: 260 }}>
          <label htmlFor={demandId}>
            <span>Client demand</span>
            <span>{demand}/min</span>
          </label>
          <input
            id={demandId}
            type="range"
            min={1}
            max={20}
            step={1}
            value={demand}
            onChange={(e) => setDemand(Number(e.target.value))}
          />
        </div>
        <button
          className="btn"
          aria-pressed={attacking}
          onClick={() => setAttacking((a) => !a)}
        >
          {attacking ? "■ Stop draining" : "⚡ Attacker drains the stock"}
        </button>
      </div>

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
        <span className="badge">Tor: one descriptor, republished on a timer</span>
        <span className="badge accent">
          Nym: single-use SURBs, replenished on consumption
        </span>
        <span className="badge">
          The directory is drawn dashed because it does not exist yet
        </span>
      </div>

      <div className="diagram-wrap">
        <svg
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          role="img"
          aria-label={`Comparison diagram. Tor: a hidden service publishes one reusable descriptor to a directory, and all ${CLIENT_COUNT} clients read the same copy. Nym: a hidden service pushes batches of single-use SURBs to a directory holding ${Math.round(stock)} of ${CAPACITY}, each client consumes a distinct SURB, and the service must push a fresh batch whenever the stock runs low.`}
        >
          {/* Tor band */}
          <text className="node-label" x={20} y={30} style={{ fontWeight: 600 }}>
            Tor: one reusable descriptor
          </text>
          <line
            x1={cols.service}
            y1={rows.tor}
            x2={cols.dir - 60}
            y2={rows.tor}
            stroke="var(--mode-vpn)"
            strokeWidth={2}
          />
          <text
            className="node-label"
            x={(cols.service + cols.dir - 60) / 2}
            y={rows.tor + 50}
            textAnchor="middle"
          >
            publishes 1x per rotation
          </text>
          {clientYs.map((dy, i) => (
            <line
              key={i}
              x1={cols.dir + 60}
              y1={rows.tor}
              x2={cols.client - 16}
              y2={rows.tor + dy}
              stroke="var(--mode-vpn)"
              strokeWidth={1.6}
              opacity={0.6}
            />
          ))}
          <text
            className="node-label"
            x={(cols.dir + cols.client) / 2}
            y={rows.tor + 50}
            textAnchor="middle"
          >
            same copy served to every client
          </text>

          <NodeGlyph kind="service-provider" x={cols.service} y={rows.tor} />
          <text className="node-label" x={cols.service} y={rows.tor + 32} textAnchor="middle">
            Hidden service
          </text>

          <DirectoryBox x={cols.dir} y={rows.tor} label="HSDir" sublabel="1 descriptor" />

          {clientYs.map((dy, i) => (
            <g key={i}>
              <NodeGlyph kind="client" x={cols.client} y={rows.tor + dy} r={9} />
              <text
                className="node-label"
                x={cols.client + 22}
                y={rows.tor + dy + 4}
              >
                client {String.fromCharCode(65 + i)}
              </text>
            </g>
          ))}

          {/* Nym band */}
          <text className="node-label" x={20} y={rows.nym - 96} style={{ fontWeight: 600 }}>
            Nym: single-use SURBs, one consumed per client
          </text>
          <line
            x1={cols.service}
            y1={rows.nym}
            x2={cols.dir - 60}
            y2={rows.nym}
            stroke="var(--mode-mixnet)"
            strokeWidth={2}
          />
          <text
            className="node-label"
            x={(cols.service + cols.dir - 60) / 2}
            y={rows.nym + 50}
            textAnchor="middle"
            style={{ fill: stockSlots <= 2 ? "var(--nym-accent)" : undefined }}
          >
            {stockSlots <= 2 ? "pushing a fresh batch" : "must keep replenishing"}
          </text>
          {clientYs.map((dy, i) => (
            <line
              key={i}
              x1={cols.dir + 60}
              y1={rows.nym}
              x2={cols.client - 16}
              y2={rows.nym + dy}
              stroke="var(--mode-mixnet)"
              strokeWidth={1.6}
              opacity={0.6}
            />
          ))}
          <text
            className="node-label"
            x={(cols.dir + cols.client) / 2}
            y={rows.nym + 50}
            textAnchor="middle"
          >
            a distinct SURB is consumed per client
          </text>

          <NodeGlyph kind="service-provider" x={cols.service} y={rows.nym} />
          <text className="node-label" x={cols.service} y={rows.nym + 32} textAnchor="middle">
            Hidden service
          </text>

          <DirectoryBox
            x={cols.dir}
            y={rows.nym}
            label='"HSDir"'
            sublabel={`stock ${Math.max(0, Math.round(stock))}/${CAPACITY}`}
            slots={stockSlots}
          />

          {clientYs.map((dy, i) => (
            <g key={i}>
              <NodeGlyph kind="client" x={cols.client} y={rows.nym + dy} r={9} />
              <text
                className="node-label"
                x={cols.client + 22}
                y={rows.nym + dy + 4}
              >
                client {String.fromCharCode(65 + i)}
              </text>
            </g>
          ))}
        </svg>
      </div>

      <div>
        <p className="section-title">What a network observer sees: uploads from the service</p>
        <div className="card">
          <UploadTrack label="Tor" history={history} pick={(t) => t.tor} accent={false} />
          <UploadTrack label="Nym" history={history} pick={(t) => t.nym} accent />
          <div className="obs-log" style={{ marginTop: 10 }}>
            <div className="obs-row">
              <span className="call">Tor uploads in window</span>
              <span className="src">{torUploads} · fixed by the rotation timer</span>
            </div>
            <div className="obs-row">
              <span className="call">Nym uploads in window</span>
              <span className="src">
                {nymUploads} · scales with demand{attacking ? " and with the attacker" : ""}
              </span>
            </div>
          </div>
          <p className="disclaimer" style={{ marginTop: 10 }}>
            Tor's track stays flat however hard clients read, because one
            descriptor serves everyone. Nym's track thickens with demand, because
            the pressure is consumption-driven. That upload pattern is the
            fingerprint that can reveal the service's network location.
          </p>
        </div>
      </div>

      <div>
        <p className="section-title">Why this is trivial to attack</p>
        <div className="card">
          <p style={{ margin: 0 }}>
            The directory dispenses a finite, single-use stock and must not learn
            who is asking, so it cannot tell whether one client is burning through
            everything. An attacker just requests SURBs in a loop. Press{" "}
            <strong>Attacker drains the stock</strong> above: the stock empties,
            the service is forced to push a fresh batch immediately, and the
            observer track lights up on command. {refills > 0 && (
              <>The service has been forced to refill <strong>{refills}</strong>{" "}
              times so far.</>
            )}
          </p>
          <p style={{ marginTop: 10, marginBottom: 0 }}>
            The refill mechanism that keeps the service reachable is the same
            mechanism that can be provoked to expose it. See{" "}
            <a href="#surb-depletion-denial-of-service">
              SURB-depletion denial of service
            </a>{" "}
            and{" "}
            <a href="#rate-limiting-anonymous-clients">
              rate-limiting anonymous clients
            </a>
            .
          </p>
        </div>
      </div>

      {reduced && (
        <p className="disclaimer">
          Reduced-motion mode: the simulation is paused. The structure, the
          directory stock, and the observer tracks are shown statically.
        </p>
      )}
    </div>
  );
}

function DirectoryBox({
  x,
  y,
  label,
  sublabel,
  slots,
}: {
  x: number;
  y: number;
  label: string;
  sublabel: string;
  slots?: number;
}) {
  const w = 120;
  const h = 56;
  return (
    <g>
      <rect
        x={x - w / 2}
        y={y - h / 2}
        width={w}
        height={h}
        rx={6}
        fill="var(--nym-bg)"
        stroke="var(--nym-text-dim)"
        strokeWidth={1.5}
        strokeDasharray="5 4"
      />
      <text className="node-label" x={x} y={y - 6} textAnchor="middle">
        {label}
      </text>
      <text
        x={x}
        y={y + 10}
        textAnchor="middle"
        style={{ fontFamily: "var(--font-mono)", fontSize: 10, fill: "var(--nym-text-dim)" }}
      >
        {sublabel}
      </text>
      {slots !== undefined &&
        Array.from({ length: 8 }, (_, i) => (
          <rect
            key={i}
            x={x - 36 + i * 9}
            y={y + 16}
            width={6}
            height={7}
            rx={1}
            fill={i < slots ? "var(--mode-mixnet)" : "var(--nym-text-dim)"}
            opacity={i < slots ? 0.9 : 0.25}
          />
        ))}
    </g>
  );
}

function UploadTrack({
  label,
  history,
  pick,
  accent,
}: {
  label: string;
  history: Tick[];
  pick: (t: Tick) => boolean;
  accent: boolean;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 6 }}>
      <span
        className="node-label"
        style={{ width: 34, fontFamily: "var(--font-mono)", fontSize: 11 }}
      >
        {label}
      </span>
      <svg
        viewBox={`0 0 ${HISTORY * 6} 22`}
        preserveAspectRatio="none"
        style={{ flex: 1, height: 22 }}
        aria-hidden="true"
      >
        <line
          x1={0}
          y1={11}
          x2={HISTORY * 6}
          y2={11}
          stroke="var(--nym-text-dim)"
          strokeWidth={1}
          opacity={0.3}
        />
        {history.map((t, i) =>
          pick(t) ? (
            <rect
              key={i}
              x={i * 6}
              y={2}
              width={3}
              height={18}
              rx={1}
              fill={accent ? "var(--nym-accent)" : "var(--mode-vpn)"}
            />
          ) : null,
        )}
      </svg>
    </div>
  );
}
