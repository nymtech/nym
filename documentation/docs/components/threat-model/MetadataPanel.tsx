"use client";

import Link from "next/link";
import type { ActorAssessment, Scenario, Tri } from "../../lib/privacy-model/types";
import { ACTORS } from "../../lib/privacy-model/threat-model";
import { actorHref } from "../../lib/privacy-model/links";
import { triClass, triGlyph } from "../../lib/privacy-model/tri";

function List({
  title,
  items,
  className,
}: {
  title: string;
  items: string[];
  className?: string;
}) {
  if (!items.length) return null;
  return (
    <div className={`meta-block ${className ?? ""}`}>
      <h4>{title}</h4>
      <ul>
        {items.map((it, i) => (
          <li key={i}>{it}</li>
        ))}
      </ul>
    </div>
  );
}

function PropChip({ id, t }: { id: string; t: Tri }) {
  return (
    <span className="badge">
      <span className={triClass(t)}>{triGlyph(t)}</span>
      {id}
    </span>
  );
}

export function ActorAssessmentCard({ a }: { a: ActorAssessment }) {
  const meta = ACTORS.find((x) => x.id === a.actor);
  return (
    <div className="card actor-card">
      <div className="actor-head">
        <Link
          href={actorHref(a.actor)}
          className="actor-link"
          title={`See ${a.actor}${meta ? ` (${meta.name})` : ""} in the threat model`}
        >
          <span className="badge accent">{a.actor}</span>
          <span className="actor-name">{meta?.name ?? a.actor}</span>
        </Link>
        <span className="actor-props">
          {a.p1 && <PropChip id="P1" t={a.p1} />}
          {a.p2 && <PropChip id="P2" t={a.p2} />}
        </span>
      </div>
      <div className="meta-grid">
        <List title="Sees" items={a.sees} className="sees" />
        <List title="Can't see" items={a.cantSee} className="cant-see" />
        <List title="Residual / countermeasure" items={a.residual} />
      </div>
    </div>
  );
}

/** Per-actor assessment for a list of actors (used by scenarios + architectures). */
export function ActorAssessmentGrid({
  assessment,
}: {
  assessment: ActorAssessment[];
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {assessment.map((a) => (
        <ActorAssessmentCard key={a.actor} a={a} />
      ))}
    </div>
  );
}

export function MetadataPanel({ scenario }: { scenario: Scenario }) {
  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexDirection: "column", gap: 12 }}
    >
      <ActorAssessmentGrid assessment={scenario.actorAssessment} />
      <div className="card">
        <div className="meta-grid">
          <List title="Pros" items={scenario.pros} />
          <List title="Cons / mitigations" items={scenario.cons} />
          <List title="Fit" items={scenario.fit} />
        </div>
      </div>
    </div>
  );
}
