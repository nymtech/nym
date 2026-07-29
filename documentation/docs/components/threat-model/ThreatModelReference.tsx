"use client";

import Link from "next/link";
import {
  ACTORS,
  AUXILIARY_INFO,
  CATEGORY_ERROR,
  PROFILE_NOTE,
  PROPERTIES,
  VECTORS,
} from "../../lib/privacy-model/threat-model";
import { actorHref } from "../../lib/privacy-model/links";

// Reference views over the generic threat-model spine. These render only from
// the spine data (../../lib/privacy-model/threat-model) and use classes already
// defined in pages/threat-model-viz.css, scoped under .nym-threat-viz.

/**
 * L1 is app-parameterised (decision D1): the spine ACTORS array holds only the
 * universal L2/L3L/L3G, and each worked example supplies what L1 observes. The
 * actors reference page still documents L1 as a concept, so its card content is
 * authored here rather than read from the spine.
 */
const L1_NOTE = {
  vantage:
    "Sees only what your application makes public, out of band from the connection itself.",
  observes:
    "Whatever is public for your application — for example, a public ledger, a directory, or a status page.",
  cannotObserve: "Anything your application does not publish.",
  cost: "Free — it reads public data.",
};

export function ActorsReference() {
  return (
    <div className="nym-threat-viz" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div className="card actor-card" id="actor-L1">
        <div className="actor-head">
          <span className="badge">L1</span>
          <span className="actor-name">Public observer</span>
          <span className="badge">Application-specific</span>
        </div>
        <p style={{ margin: "0 0 8px", color: "var(--nym-text-dim)" }}>
          {L1_NOTE.vantage}
        </p>
        <div className="meta-grid">
          <div className="meta-block sees">
            <h4>Observes</h4>
            <ul>
              <li>{L1_NOTE.observes}</li>
            </ul>
          </div>
          <div className="meta-block cant-see">
            <h4>Cannot observe</h4>
            <ul>
              <li>{L1_NOTE.cannotObserve}</li>
            </ul>
          </div>
          <div className="meta-block">
            <h4>Cost</h4>
            <ul>
              <li>{L1_NOTE.cost}</li>
            </ul>
          </div>
        </div>
      </div>

      {ACTORS.map((a) => (
        <div className="card actor-card" id={`actor-${a.id}`} key={a.id}>
          <div className="actor-head">
            <span className={`badge ${a.primary ? "accent" : ""}`}>{a.id}</span>
            <span className="actor-name">{a.name}</span>
            {a.primary && <span className="badge accent">Primary adversary</span>}
          </div>
          <p style={{ margin: "0 0 8px", color: "var(--nym-text-dim)" }}>
            {a.vantage}
          </p>
          <div className="meta-grid">
            <div className="meta-block sees">
              <h4>Observes</h4>
              <ul>
                {a.observes.map((o, i) => (
                  <li key={i}>{o}</li>
                ))}
              </ul>
            </div>
            <div className="meta-block cant-see">
              <h4>Cannot observe</h4>
              <ul>
                {a.cannotObserve.map((o, i) => (
                  <li key={i}>{o}</li>
                ))}
              </ul>
            </div>
            <div className="meta-block">
              <h4>Cost</h4>
              <ul>
                <li>{a.cost}</li>
              </ul>
            </div>
          </div>
        </div>
      ))}

      <p className="disclaimer" style={{ marginTop: 4 }}>
        {AUXILIARY_INFO}
      </p>
    </div>
  );
}

export function VectorsReference() {
  return (
    <div className="nym-threat-viz" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {VECTORS.map((v) => (
        <div className="card" id={`vector-${v.id}`} key={v.id} style={{ scrollMarginTop: 84 }}>
          <div className="actor-head">
            <span className="badge accent">{v.id}</span>
            <span className="actor-name">{v.name}</span>
          </div>
          <p style={{ margin: "8px 0", color: "var(--nym-text-dim)" }}>
            {v.consistsOf}
          </p>
          <div className="meta-grid">
            <div className="meta-block">
              <h4>Observable from</h4>
              <ul>
                {v.observableFrom.map((actor) => (
                  <li key={actor}>
                    <Link href={actorHref(actor)} className="prop-link">
                      {actor}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
            <div className="meta-block">
              <h4>Countermeasures</h4>
              <ul>
                {v.countermeasures.map((c, i) => (
                  <li key={i}>
                    <span className="badge">
                      {c.layer === "hygiene" ? "Layer 2" : "Layer 1"} · vs {c.against.join("/")}
                    </span>{" "}
                    {c.text}
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      ))}
      <p className="disclaimer" style={{ marginTop: 4 }}>
        {CATEGORY_ERROR}
      </p>
    </div>
  );
}

export function PropertiesReference() {
  return (
    <div className="nym-threat-viz" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div className="meta-grid">
        {PROPERTIES.map((p) => (
          <div className="card" id={`prop-${p.id}`} key={p.id} style={{ scrollMarginTop: 84 }}>
            <div className="actor-head">
              <span className="badge accent">{p.id}</span>
              <span className="actor-name">{p.name}</span>
            </div>
            <p style={{ margin: "8px 0 0", color: "var(--nym-text-dim)", fontSize: "0.9rem" }}>
              {p.definition}
            </p>
          </div>
        ))}
      </div>
      <p className="disclaimer" style={{ marginTop: 4 }}>
        {PROFILE_NOTE}
      </p>
    </div>
  );
}
