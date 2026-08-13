"use client";

import Link from "next/link";
import type { ActorId, Scenario, Tri } from "lib/privacy-model/types";
import { triClass, triGlyph } from "lib/privacy-model/tri";
import { actorHref } from "lib/privacy-model/links";

/** Compact per-actor verdict row for a scenario, plus the performance badge. */
export function PropertyBadges({ scenario }: { scenario: Scenario }) {
  const { matrix, performance } = scenario;
  const items: { pre: string; prop: string; actor: ActorId; t: Tri }[] = [
    { pre: "IP hidden", prop: "P1", actor: "L2", t: matrix.p1L2.verdict },
    { pre: "Req-unlink", prop: "P2", actor: "L2", t: matrix.p2L2.verdict },
    { pre: "Local net", prop: "P1", actor: "L3L", t: matrix.p1L3L.verdict },
    { pre: "Global net", prop: "P1", actor: "L3G", t: matrix.p1L3G.verdict },
  ];
  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexWrap: "wrap", gap: 8 }}
    >
      {items.map(({ pre, prop, actor, t }) => (
        <span className="badge" key={`${pre}-${actor}`}>
          <span className={triClass(t)}>{triGlyph(t)}</span>
          {pre} ·{" "}
          <Link
            href={actorHref(actor)}
            className="prop-link"
            title={`See ${actor} in the threat model`}
          >
            {prop}@{actor}
          </Link>
        </span>
      ))}
      <span className="badge" title={performance.note}>
        <span className={triClass(performance.fastSync)}>
          {triGlyph(performance.fastSync)}
        </span>
        Fast
      </span>
    </div>
  );
}
