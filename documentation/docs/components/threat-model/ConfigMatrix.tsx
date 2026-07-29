"use client";

import Link from "next/link";
import { GENERIC_SCENARIOS } from "../../lib/privacy-model/examples/generic";
import { triClass, triGlyph } from "../../lib/privacy-model/tri";
import { actorHref } from "../../lib/privacy-model/links";
import type { MatrixCell } from "../../lib/privacy-model/types";

// The cross-configuration comparison matrix: one row per generic configuration,
// projecting each scenario's `matrix` verdicts into a single grid. Rows link to
// the per-configuration detail pages; column headers link to the threat actors.

function VerdictCell({ c }: { c: MatrixCell }) {
  return (
    <td>
      <span className={triClass(c.verdict)}>{triGlyph(c.verdict)}</span>
      <span className="matrix-col-sub">{c.text}</span>
    </td>
  );
}

export function ConfigMatrix() {
  return (
    <div className="nym-threat-viz">
      <div className="diagram-wrap" style={{ padding: 4 }}>
        <table className="matrix">
          <thead>
            <tr>
              <th style={{ textAlign: "left" }}>Configuration</th>
              <th>
                IP hidden
                <span className="matrix-col-sub">
                  P1 @ <Link href={actorHref("L2")} className="prop-link">L2</Link>
                </span>
              </th>
              <th>
                Requests unlinkable
                <span className="matrix-col-sub">
                  P2 @ <Link href={actorHref("L2")} className="prop-link">L2</Link>
                </span>
              </th>
              <th>
                Local network
                <span className="matrix-col-sub">
                  P1 @ <Link href={actorHref("L3L")} className="prop-link">L3L</Link>
                </span>
              </th>
              <th>
                Global network
                <span className="matrix-col-sub">
                  P1 @ <Link href={actorHref("L3G")} className="prop-link">L3G</Link>
                </span>
              </th>
              <th>Fast</th>
            </tr>
          </thead>
          <tbody>
            {GENERIC_SCENARIOS.map((s) => (
              <tr key={s.id} className={s.recommended ? "recommended" : ""}>
                <td className="name">
                  <Link href={`/network/threat-model/configurations/${s.id}`}>
                    {s.recommended ? "★ " : ""}
                    {s.shortTitle}
                  </Link>
                </td>
                <VerdictCell c={s.matrix.p1L2} />
                <VerdictCell c={s.matrix.p2L2} />
                <VerdictCell c={s.matrix.p1L3L} />
                <VerdictCell c={s.matrix.p1L3G} />
                <td>
                  <span className={triClass(s.performance.fastSync)}>
                    {triGlyph(s.performance.fastSync)}
                  </span>
                  {s.performance.note && (
                    <span className="matrix-col-sub">{s.performance.note}</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
