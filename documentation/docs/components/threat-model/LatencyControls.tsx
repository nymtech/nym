"use client";

import { GEO_BANDS, type LatencyParams } from "../../lib/privacy-model/latency";

export function LatencyControls({
  params,
  onChange,
  showMixing = true,
}: {
  params: LatencyParams;
  onChange: (p: LatencyParams) => void;
  /** Mixing delay only applies to mixnet routes; hide it otherwise. */
  showMixing?: boolean;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <div className="slider-row">
        <label>
          <span>Geographic delay / hop</span>
          <span>{params.geoMsPerHop} ms</span>
        </label>
        <input
          type="range"
          min={40}
          max={300}
          step={10}
          value={params.geoMsPerHop}
          onChange={(e) =>
            onChange({ ...params, geoMsPerHop: Number(e.target.value) })
          }
        />
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            fontSize: "0.68rem",
            color: "var(--nym-text-faint)",
          }}
        >
          {GEO_BANDS.map((b) => (
            <span key={b.id}>
              {b.label} ({b.ms})
            </span>
          ))}
        </div>
      </div>

      {showMixing && (
        <div className="slider-row">
          <label>
            <span>Mixing delay / layer</span>
            <span>{params.mixDelayMs} ms</span>
          </label>
          <input
            type="range"
            min={0}
            max={250}
            step={10}
            value={params.mixDelayMs}
            onChange={(e) =>
              onChange({ ...params, mixDelayMs: Number(e.target.value) })
            }
          />
        </div>
      )}
    </div>
  );
}
