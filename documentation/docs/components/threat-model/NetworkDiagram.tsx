"use client";

import { useMemo } from "react";
import { motion } from "framer-motion";
import { scalePoint } from "d3-scale";
import type {
  Architecture,
  DiagramRoute,
  Mode,
  Scenario,
} from "../../lib/privacy-model/types";
import { MODE_META } from "../../lib/privacy-model/nodes";
import { buildTopology } from "../../lib/privacy-model/topology";
import {
  DEFAULT_LATENCY_PARAMS,
  type LatencyParams,
} from "../../lib/privacy-model/latency";
import { useReducedMotion } from "../../lib/privacy-model/useReducedMotion";
import { NodeGlyph } from "./NodeGlyph";

const WIDTH = 940;
const PAD_X = 95;
const V_GAP = 78;
const MIX_PER_LAYER = 6; // expand each mix layer into this many nodes
const MIX_GAP = 30;
const TOP = 64;
const BOTTOM = 48;
const BOX_W = 178;

interface Pos {
  x: number;
  y: number;
}

function routeColor(r: DiagramRoute): string {
  return r.surb ? "var(--nym-accent)" : `var(${MODE_META[r.mode].colorVar})`;
}
function routeDashed(r: DiagramRoute): boolean {
  return r.surb ? true : MODE_META[r.mode].dashed;
}
function routeStyleKey(r: DiagramRoute): string {
  return r.surb ? "surb" : r.mode;
}
function routeStyleLabel(r: DiagramRoute): string {
  return r.surb ? "SURB reply" : MODE_META[r.mode].label;
}

/** Deterministic mix-node pick per packet+layer (SSR-safe, spreads packets). */
function mixPick(nodes: Pos[], seed: number, ord: number): Pos {
  return nodes[(seed * 2 + ord * 3 + 1) % nodes.length];
}

export function NetworkDiagram({
  item,
  params = DEFAULT_LATENCY_PARAMS,
  animate = true,
  activeMode,
}: {
  item: Scenario | Architecture;
  params?: LatencyParams;
  animate?: boolean;
  /** If set, only routes of this transport mode are active; others are dimmed. */
  activeMode?: Mode;
}) {
  const reduced = useReducedMotion();
  const doAnimate = animate && !reduced;
  const topo = useMemo(() => buildTopology(item), [item]);

  const { pos, kindMap, colMap, mixLayers, mixColOrder, height } = useMemo(() => {
    const cols = Array.from(new Set(topo.nodes.map((n) => n.col))).sort(
      (a, b) => a - b,
    );
    const x = scalePoint<number>()
      .domain(cols)
      .range([PAD_X, WIDTH - PAD_X]);
    const perCol = new Map<number, typeof topo.nodes>();
    topo.nodes.forEach((n) => {
      const arr = perCol.get(n.col) ?? [];
      arr.push(n);
      perCol.set(n.col, arr);
    });
    // Columns that hold mix nodes become expanded mixnet layers.
    const mixColOrder = cols.filter((c) =>
      topo.nodes.some((n) => n.col === c && n.kind === "mix"),
    );
    const nonMixCounts = cols
      .filter((c) => !mixColOrder.includes(c))
      .map((c) => perCol.get(c)?.length ?? 1);
    const maxNonMix = Math.max(1, ...nonMixCounts);
    const h = Math.max(
      280,
      TOP + (MIX_PER_LAYER - 1) * MIX_GAP + BOTTOM,
      TOP + (maxNonMix - 1) * V_GAP + BOTTOM,
    );
    const centerY = h / 2;

    const pos = new Map<string, Pos>();
    const kindMap = new Map<string, string>();
    const colMap = new Map<string, number>();
    const mixLayers = new Map<number, Pos[]>();

    perCol.forEach((arr, col) => {
      const cx = x(col) ?? PAD_X;
      if (mixColOrder.includes(col)) {
        const nodes: Pos[] = [];
        for (let k = 0; k < MIX_PER_LAYER; k++) {
          nodes.push({ x: cx, y: centerY + (k - (MIX_PER_LAYER - 1) / 2) * MIX_GAP });
        }
        mixLayers.set(col, nodes);
        // representative position (layer centre) for the mix node id(s)
        arr.forEach((n) => {
          pos.set(n.id, { x: cx, y: centerY });
          kindMap.set(n.id, n.kind);
          colMap.set(n.id, col);
        });
      } else {
        arr.forEach((n, k) => {
          pos.set(n.id, { x: cx, y: centerY + (k - (arr.length - 1) / 2) * V_GAP });
          kindMap.set(n.id, n.kind);
          colMap.set(n.id, col);
        });
      }
    });
    return { pos, kindMap, colMap, mixLayers, mixColOrder, height: h };
  }, [topo]);

  // Faint mixnet mesh: fans (entry↔layer, layer↔exit) and full inter-layer mesh.
  const mesh = useMemo(() => {
    const lines: { x1: number; y1: number; x2: number; y2: number; dim: boolean }[] = [];
    const seen = new Set<string>();
    const add = (a: Pos, b: Pos, dim: boolean) => {
      const key = `${Math.round(a.x)},${Math.round(a.y)},${Math.round(b.x)},${Math.round(b.y)}`;
      if (seen.has(key)) return;
      seen.add(key);
      lines.push({ x1: a.x, y1: a.y, x2: b.x, y2: b.y, dim });
    };
    topo.routes.forEach((r) => {
      const dim = !!activeMode && r.mode !== activeMode;
      for (let i = 0; i < r.nodeIds.length - 1; i++) {
        const aId = r.nodeIds[i];
        const bId = r.nodeIds[i + 1];
        const aMix = kindMap.get(aId) === "mix";
        const bMix = kindMap.get(bId) === "mix";
        if (!aMix && !bMix) continue;
        const layerA = aMix ? mixLayers.get(colMap.get(aId)!)! : [pos.get(aId)!];
        const layerB = bMix ? mixLayers.get(colMap.get(bId)!)! : [pos.get(bId)!];
        for (const pa of layerA) for (const pb of layerB) add(pa, pb, dim);
      }
    });
    return lines;
  }, [topo, pos, mixLayers, colMap, kindMap, activeMode]);

  // Non-mix edges only (mix segments are drawn by the mesh above).
  const edges = useMemo(() => {
    const isMixSeg = (a: string, b: string) =>
      kindMap.get(a) === "mix" || kindMap.get(b) === "mix";
    const groups = new Map<string, string[]>();
    topo.routes.forEach((r) => {
      const style = routeStyleKey(r);
      for (let i = 0; i < r.nodeIds.length - 1; i++) {
        const aId = r.nodeIds[i];
        const bId = r.nodeIds[i + 1];
        if (isMixSeg(aId, bId)) continue;
        const A = pos.get(aId)!;
        const B = pos.get(bId)!;
        const span = Math.abs(colMap.get(aId)! - colMap.get(bId)!);
        if (r.orthogonal && A.y !== B.y && span > 1) continue;
        const segKey = [aId, bId].sort().join("~");
        const styles = groups.get(segKey) ?? [];
        if (!styles.includes(style)) styles.push(style);
        groups.set(segKey, styles);
      }
    });

    const items: { pts: Pos[]; color: string; dashed: boolean; dim: boolean }[] = [];
    const seen = new Set<string>();
    topo.routes.forEach((r) => {
      const style = routeStyleKey(r);
      const color = routeColor(r);
      const dashed = routeDashed(r);
      const dim = !!activeMode && r.mode !== activeMode;
      for (let i = 0; i < r.nodeIds.length - 1; i++) {
        const aId = r.nodeIds[i];
        const bId = r.nodeIds[i + 1];
        if (isMixSeg(aId, bId)) continue;
        const A = pos.get(aId)!;
        const B = pos.get(bId)!;
        const span = Math.abs(colMap.get(aId)! - colMap.get(bId)!);
        if (r.orthogonal && A.y !== B.y && span > 1) {
          items.push({ pts: [A, { x: A.x, y: B.y }, B], color, dashed, dim });
        } else {
          const segKey = [aId, bId].sort().join("~");
          const dk = `${segKey}:${style}:${dim ? "d" : "a"}`;
          if (seen.has(dk)) continue;
          seen.add(dk);
          const styles = groups.get(segKey) ?? [style];
          const idx = styles.indexOf(style);
          const off = (idx - (styles.length - 1) / 2) * 5;
          const dx = B.x - A.x;
          const dy = B.y - A.y;
          const len = Math.hypot(dx, dy) || 1;
          const nx = (-dy / len) * off;
          const ny = (dx / len) * off;
          items.push({
            pts: [
              { x: A.x + nx, y: A.y + ny },
              { x: B.x + nx, y: B.y + ny },
            ],
            color,
            dashed,
            dim,
          });
        }
      }
    });
    return items;
  }, [topo, pos, colMap, kindMap, activeMode]);

  const packetSpecs = useMemo(() => {
    const many = topo.routes.length > 2;
    const specs: {
      route: DiagramRoute;
      routeId: string;
      k: number;
      download: boolean;
      delay: number;
      seed: number;
    }[] = [];
    const routes = activeMode
      ? topo.routes.filter((r) => r.mode === activeMode)
      : topo.routes;
    // Same duration a packet takes to traverse a route as in PacketDot below,
    // so we can delay a response until its request has actually arrived.
    const mixNodeIds = new Set(
      topo.nodes.filter((n) => n.kind === "mix").map((n) => n.id),
    );
    const routeDuration = (r: DiagramRoute): number => {
      const mixish = r.mode === "mixnet";
      const mixCount = r.nodeIds.filter((id) => mixNodeIds.has(id)).length;
      return (
        (r.surb ? 3.0 : mixish ? 3.2 : r.mode === "dvpn" ? 2.2 : 2.4) +
        mixCount * 0.4
      );
    };

    const GAP = 0.7;
    let routeBase = 0;
    routes.forEach((r) => {
      const dir = r.direction ?? "upload";
      let count: number;
      let downloads: number;
      if (dir === "upload") {
        count = many ? 1 : 2;
        downloads = 0;
      } else if (dir === "download") {
        count = many ? 1 : 2;
        downloads = count;
      } else {
        count = many ? 3 : 5;
        const up = Math.max(1, Math.round(count * (1 - (r.downloadRatio ?? 0.85))));
        downloads = Math.max(1, count - up);
      }
      const uploads = count - downloads;
      // Requests (uploads) fire first, staggered from the route's base delay.
      for (let u = 0; u < uploads; u++) {
        specs.push({
          route: r,
          routeId: r.id,
          k: u,
          download: false,
          delay: routeBase + u * GAP,
          seed: specs.length,
        });
      }
      // Responses (downloads) start only after the first request has travelled
      // all the way to the destination (its full traversal time), not merely a
      // slot later — so IPR→client never precedes client→…→destination.
      const dlBase = uploads > 0 ? routeBase + routeDuration(r) + 0.3 : routeBase;
      for (let d = 0; d < downloads; d++) {
        specs.push({
          route: r,
          routeId: r.id,
          k: uploads + d,
          download: true,
          delay: dlBase + d * GAP,
          seed: specs.length,
        });
      }
      routeBase += GAP;
    });
    return specs;
  }, [topo, activeMode]);

  const mixDim = !!activeMode && activeMode !== "mixnet";
  const layerLabelY =
    height / 2 - ((MIX_PER_LAYER - 1) / 2) * MIX_GAP - 24;

  return (
    <div className="nym-threat-viz">
      {/* route-style legend */}
      <div className="legend" style={{ marginBottom: 8, fontSize: "0.76rem" }}>
        {dedupeRoutes(topo.routes).map((r) => (
          <span className="legend-item" key={routeStyleKey(r)}>
            <span
              className="legend-swatch"
              style={{
                background: routeColor(r),
                height: 3,
                width: 22,
                borderRadius: 0,
                border: routeDashed(r)
                  ? "1px dashed var(--nym-text-faint)"
                  : "none",
              }}
            />
            {routeStyleLabel(r)}
          </span>
        ))}
      </div>

      <div className="diagram-wrap">
        <svg viewBox={`0 0 ${WIDTH} ${height}`} role="img">
          {/* mixnet mesh */}
          <g stroke="var(--mode-mixnet)" strokeWidth={1}>
            {mesh.map((l, i) => (
              <line
                key={i}
                x1={l.x1}
                y1={l.y1}
                x2={l.x2}
                y2={l.y2}
                opacity={l.dim ? 0.05 : 0.16}
              />
            ))}
          </g>

          {/* non-mix edges */}
          {edges.map((e, i) => (
            <polyline
              key={i}
              points={e.pts.map((p) => `${p.x},${p.y}`).join(" ")}
              fill="none"
              stroke={e.color}
              strokeWidth={2}
              strokeDasharray={e.dashed ? "6 5" : undefined}
              opacity={e.dim ? 0.12 : 0.8}
            />
          ))}

          {/* packets */}
          {doAnimate &&
            packetSpecs.map((s) => (
              <Packet
                key={`${s.routeId}-${s.k}`}
                route={s.route}
                pos={pos}
                kindMap={kindMap}
                colMap={colMap}
                mixLayers={mixLayers}
                mixColOrder={mixColOrder}
                seed={s.seed}
                delay={s.delay}
                download={s.download}
              />
            ))}

          {/* mix layers (expanded) */}
          {mixColOrder.map((col, ord) => {
            const nodes = mixLayers.get(col)!;
            return (
              <g key={`mix-${col}`} opacity={mixDim ? 0.3 : 1}>
                <text
                  x={nodes[0].x}
                  y={layerLabelY}
                  textAnchor="middle"
                  style={{ fontFamily: "var(--font-mono)", fontSize: 10, fill: "var(--nym-text-faint)" }}
                >
                  Mix L{ord + 1}
                </text>
                <text
                  x={nodes[0].x}
                  y={layerLabelY + 13}
                  textAnchor="middle"
                  style={{ fontFamily: "var(--font-mono)", fontSize: 10, fill: "var(--mode-mixnet)" }}
                >
                  +{params.mixDelayMs} ms
                </text>
                {nodes.map((p, i) => (
                  <NodeGlyph key={i} kind="mix" x={p.x} y={p.y} r={8} />
                ))}
              </g>
            );
          })}

          {/* non-mix nodes */}
          {topo.nodes.map((n) => {
            if (n.kind === "mix") return null;
            const p = pos.get(n.id)!;
            if (n.components && n.components.length) {
              return (
                <ComponentBox
                  key={n.id}
                  x={p.x}
                  y={p.y}
                  title={n.label ?? "Client"}
                  components={n.components}
                />
              );
            }
            const isObserver =
              n.kind === "destination" || n.kind === "service-provider";
            return (
              <g key={n.id}>
                {isObserver && (
                  <circle className="observer-halo" cx={p.x} cy={p.y} r={20} />
                )}
                <NodeGlyph kind={n.kind} x={p.x} y={p.y} />
                <text className="node-label" x={p.x} y={p.y + 30} textAnchor="middle">
                  {n.label ?? shortLabel(n.kind)}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      {topo.caption && (
        <p className="disclaimer" style={{ marginTop: 8 }}>
          {topo.caption}
        </p>
      )}
    </div>
  );
}

function Packet({
  route,
  pos,
  kindMap,
  colMap,
  mixLayers,
  mixColOrder,
  seed,
  delay,
  download,
}: {
  route: DiagramRoute;
  pos: Map<string, Pos>;
  kindMap: Map<string, string>;
  colMap: Map<string, number>;
  mixLayers: Map<number, Pos[]>;
  mixColOrder: number[];
  seed: number;
  delay: number;
  download: boolean;
}) {
  const mixish = route.mode === "mixnet";
  // Resolve each stage: mix stages route through a picked node of that layer.
  const raw: Pos[] = route.nodeIds.map((id) => {
    if (kindMap.get(id) === "mix") {
      const col = colMap.get(id)!;
      return mixPick(mixLayers.get(col)!, seed, mixColOrder.indexOf(col));
    }
    return pos.get(id)!;
  });

  const pts: Pos[] = [];
  route.nodeIds.forEach((id, i) => {
    const cur = raw[i];
    if (i > 0 && route.orthogonal) {
      const prev = raw[i - 1];
      const span = Math.abs(colMap.get(route.nodeIds[i - 1])! - colMap.get(id)!);
      if (prev.y !== cur.y && span > 1) pts.push({ x: prev.x, y: cur.y }); // elbow
    }
    pts.push(cur);
    if (mixish && kindMap.get(id) === "mix") pts.push({ x: cur.x, y: cur.y }); // dwell
  });
  if (download) pts.reverse();
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  const times = xs.map((_, i) => i / (xs.length - 1));
  const opacity = xs.map((_, i) => (i === 0 || i === xs.length - 1 ? 0 : 1));
  const mixCount = mixish
    ? route.nodeIds.filter((id) => kindMap.get(id) === "mix").length
    : 0;
  const duration =
    (route.surb ? 3.0 : mixish ? 3.2 : route.mode === "dvpn" ? 2.2 : 2.4) +
    mixCount * 0.4;

  const solid = download || route.surb;
  const color = routeColor(route);
  return (
    <motion.circle
      r={route.surb ? 3.5 : 4.5}
      fill={solid ? color : "var(--nym-bg)"}
      stroke={color}
      strokeWidth={solid ? 0 : 1.5}
      initial={{ cx: xs[0], cy: ys[0], opacity: 0 }}
      animate={{ cx: xs, cy: ys, opacity }}
      transition={{
        duration,
        times,
        ease: "linear",
        repeat: Infinity,
        delay,
        repeatDelay: 0.25,
      }}
    />
  );
}

function ComponentBox({
  x,
  y,
  title,
  components,
}: {
  x: number;
  y: number;
  title: string;
  components: string[];
}) {
  const lineH = 17;
  const headH = 22;
  const padY = 9;
  const h = headH + components.length * lineH + padY;
  const left = x - BOX_W / 2;
  const top = y - h / 2;
  return (
    <g>
      <rect
        x={left}
        y={top}
        width={BOX_W}
        height={h}
        fill="var(--nym-surface-2)"
        stroke="var(--nym-accent)"
        strokeWidth={1.5}
      />
      <rect x={left} y={top} width={4} height={h} fill="var(--nym-accent)" />
      <text
        x={x}
        y={top + 15}
        textAnchor="middle"
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 700,
          fill: "var(--nym-text)",
        }}
      >
        {title}
      </text>
      {components.map((c, i) => (
        <text
          key={i}
          x={x}
          y={top + headH + 8 + i * lineH}
          textAnchor="middle"
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 9.5,
            fill: "var(--nym-text-dim)",
          }}
        >
          {c}
        </text>
      ))}
    </g>
  );
}

function dedupeRoutes(routes: DiagramRoute[]): DiagramRoute[] {
  const seen = new Set<string>();
  const out: DiagramRoute[] = [];
  for (const r of routes) {
    const k = routeStyleKey(r);
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(r);
  }
  return out;
}

function shortLabel(kind: string): string {
  switch (kind) {
    case "client":
      return "Client";
    case "vpn":
      return "VPN";
    case "entry":
      return "Entry GW";
    case "mix":
      return "Mix";
    case "exit":
      return "Exit GW";
    case "ipr":
      return "IPR";
    case "service-provider":
      return "Service prov.";
    case "destination":
      return "Destination";
    case "internet":
      return "Internet";
    default:
      return kind;
  }
}
