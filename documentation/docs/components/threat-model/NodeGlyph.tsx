"use client";

import type { NodeKind } from "lib/privacy-model/types";
import { NODE_META } from "lib/privacy-model/nodes";

export function NodeGlyph({
  kind,
  x,
  y,
  r = 13,
}: {
  kind: NodeKind;
  x: number;
  y: number;
  r?: number;
}) {
  const meta = NODE_META[kind];
  const fill = `var(${meta.colorVar})`;
  const common = {
    fill,
    stroke: "var(--nym-bg)",
    strokeWidth: 1.5,
  };

  switch (meta.shape) {
    case "circle":
      return <circle cx={x} cy={y} r={r} {...common} />;
    case "square":
      return (
        <rect
          x={x - r}
          y={y - r}
          width={r * 2}
          height={r * 2}
          {...common}
        />
      );
    case "diamond":
      return (
        <polygon
          points={`${x},${y - r} ${x + r},${y} ${x},${y + r} ${x - r},${y}`}
          {...common}
        />
      );
    case "hex": {
      const pts = hexPoints(x, y, r);
      return <polygon points={pts} {...common} />;
    }
  }
}

function hexPoints(cx: number, cy: number, r: number): string {
  const pts: string[] = [];
  for (let i = 0; i < 6; i++) {
    const a = (Math.PI / 3) * i - Math.PI / 6;
    pts.push(`${cx + r * Math.cos(a)},${cy + r * Math.sin(a)}`);
  }
  return pts.join(" ");
}
