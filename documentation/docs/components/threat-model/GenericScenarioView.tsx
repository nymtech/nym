"use client";

import type { Scenario } from "../../lib/privacy-model/types";
import { NetworkDiagram } from "./NetworkDiagram";
import { PropertyBadges } from "./PropertyBadges";
import { MetadataPanel } from "./MetadataPanel";

// Per-configuration detail: the at-a-glance verdict badges, the animated path
// diagram, and the per-actor assessment. Built from a single generic scenario.
export function GenericScenarioView({ scenario }: { scenario: Scenario }) {
  return (
    <div
      className="nym-threat-viz"
      style={{ display: "flex", flexDirection: "column", gap: 18 }}
    >
      <PropertyBadges scenario={scenario} />
      <NetworkDiagram item={scenario} />
      <MetadataPanel scenario={scenario} />
      <p className="disclaimer">
        Verdicts and the latency implied by the path are an illustrative model,
        not measured values.
      </p>
    </div>
  );
}
