// Shared domain types for the privacy model. Scenario/example content is the
// single source of truth for the diagrams, the comparison matrix, and the
// per-actor assessment panels.
//
// Ported from the zcash-wallet-visualisations lib and generalised: the
// destination is a generic "destination" node whose display label each worked
// example supplies through `nodeLabels`, so the engine carries no
// application-specific names.

/** A node that can appear on a data path. */
export type NodeKind =
  | "client"
  | "vpn"
  | "entry" // Nym entry gateway
  | "mix" // Nym mix node (rendered expanded in the deep-dive)
  | "exit" // Nym exit gateway
  | "ipr" // IP Packet Router (exit-side)
  | "service-provider" // Nym-native service (SURB replies)
  | "destination" // the observing server the client talks to
  | "internet"; // open internet hop

/** Transport applied to a route segment. Drives styling + latency. */
export type Mode = "direct" | "vpn" | "dvpn" | "mixnet";

/** Tri-state used for the summary matrix (✅/❌/partial). */
export type Tri = "yes" | "no" | "partial";

/**
 * Shared-node topology for branching scenarios (multipath, hybrid): nodes are
 * placed in columns and referenced by id, so a single client / entry gateway /
 * destination can be shared across several routes instead of duplicated.
 */
export interface DiagramNode {
  id: string;
  kind: NodeKind;
  col: number;
  label?: string;
  /** If set, the node is drawn as a container box listing internal components. */
  components?: string[];
}

export interface DiagramRoute {
  id: string;
  label?: string;
  mode: Mode;
  /** Ordered node ids the route traverses. */
  nodeIds: string[];
  note?: string;
  /** Return path via SURBs (pre-computed reply routes) — drawn distinctly. */
  surb?: boolean;
  /** Route long spans (>1 column) with right-angled (elbow) connectors. */
  orthogonal?: boolean;
  /**
   * Direction of animated traffic:
   *  - "upload"   client → destination (default)
   *  - "download" destination → client (replies flowing back)
   *  - "mixed"    mostly download with some upload (see downloadRatio)
   */
  direction?: TrafficDirection;
  /** For direction "mixed": fraction of packets that are downloads (0..1). */
  downloadRatio?: number;
}

export type TrafficDirection = "upload" | "download" | "mixed";

export interface Topology {
  nodes: DiagramNode[];
  routes: DiagramRoute[];
  /** Optional caption rendered under the diagram. */
  caption?: string;
}

/** One route through the network (a scenario may have several concurrent). */
export interface RoutePath {
  id: string;
  /** e.g. "Block sync" / "Tx broadcast"; omit for single-path scenarios. */
  label?: string;
  mode: Mode;
  /** Ordered node kinds, source → observation point. */
  stages: NodeKind[];
  note?: string;
  /** Traffic direction for the animation (see DiagramRoute.direction). */
  direction?: TrafficDirection;
  /** For direction "mixed": fraction of packets that are downloads (0..1). */
  downloadRatio?: number;
}

// Threat model (generic spine; Claudia Diaz note §2–§3)

/**
 * Named threat actor (adversary level).
 * L2 (the destination) is the universal spine. L1 (a public/out-of-band
 * observer) is optional and supplied per example, because "what is public"
 * depends on the application.
 */
export type ActorId = "L1" | "L2" | "L3L" | "L3G";
/** Linkage vector. */
export type VectorId = "V1" | "V2" | "V3";
/** Unlinkability property. */
export type PropertyId = "P1" | "P2";
/** Invariant that must hold — defined per worked example. */
export type InvariantId = string;
/** Which of the two layers a countermeasure lives in. */
export type Layer = "transport" | "hygiene";

export interface ThreatActor {
  id: ActorId;
  name: string;
  /** Where the actor sits / what it controls. */
  vantage: string;
  observes: string[];
  cannotObserve: string[];
  /** How cheap/hard it is to instantiate. */
  cost: string;
  /** True for the primary adversary (L2). */
  primary?: boolean;
}

export interface Countermeasure {
  text: string;
  layer: Layer;
  /** Which adversary/adversaries this countermeasure defends against. */
  against: ActorId[];
}

export interface Vector {
  id: VectorId;
  name: string;
  consistsOf: string;
  observableFrom: ActorId[];
  countermeasures: Countermeasure[];
}

export interface Property {
  id: PropertyId;
  name: string;
  definition: string;
}

export interface Invariant {
  id: InvariantId;
  name: string;
  statement: string;
  /** What delivering this invariant depends on. */
  dependsOn: string;
}

/** Performance descriptor — orthogonal to privacy. */
export interface Performance {
  fastSync: Tri;
  note?: string;
}

/**
 * One cell of the comparison matrix: a verdict glyph plus the conditional
 * detail text (e.g. "given per-request rotation") that a bare yes/no loses.
 */
export interface MatrixCell {
  verdict: Tri;
  text: string;
}

/** The matrix row for a scenario/config (privacy only). */
export interface ScenarioMatrix {
  /** P1 via V1 at L2 — is the client IP hidden from the destination? */
  p1L2: MatrixCell;
  /** P2 via V1 at L2 — are requests unlinkable to each other at the destination? */
  p2L2: MatrixCell;
  /** P1 via V2 at L3L — local network observer. */
  p1L3L: MatrixCell;
  /** P1 via V2 at L3G — global network observer. */
  p1L3G: MatrixCell;
}

/** Per-scenario assessment against one threat actor (for the detail panel). */
export interface ActorAssessment {
  actor: ActorId;
  sees: string[];
  cantSee: string[];
  p1?: Tri;
  p2?: Tri;
  /** Residual leakage and the countermeasure that addresses it. */
  residual: string[];
}

export interface Scenario {
  id: string;
  kind: "scenario";
  title: string;
  shortTitle: string;
  summary: string;
  paths: RoutePath[];
  /** Privacy matrix row. */
  matrix: ScenarioMatrix;
  /** Performance (throughput) — shown as a separate strip, not a privacy column. */
  performance: Performance;
  /** What the configuration requires to be deployed. */
  requires: string;
  /** Per-actor assessment threaded through L2/L3L/L3G. */
  actorAssessment: ActorAssessment[];
  pros: string[];
  cons: string[];
  fit: string[];
  recommended?: boolean;
  topology?: Topology;
  /** Per-example display labels for node kinds (e.g. destination → "lightwalletd"). */
  nodeLabels?: Partial<Record<NodeKind, string>>;
}

export interface Architecture {
  id: string;
  kind: "architecture";
  title: string;
  shortTitle: string;
  runtime: string;
  modes: Mode[];
  sendOverIt: string[];
  speed: string;
  summary: string;
  notes: string[];
  paths: RoutePath[];
  recommended?: boolean;
  topology?: Topology;
  /** If set, render the client data-flow pipeline for this transport mode. */
  showClientPipeline?: Mode;
  /**
   * If set, the diagram offers a dVPN/mixnet mode toggle (only the selected
   * mode's path is active); a second "app tunnelling" view shows both at once.
   */
  modeToggle?: boolean;
  /** Scenario id whose per-actor assessment this architecture inherits. */
  inheritsAssessment?: string;
  /** For mode-toggle architectures: the scenario id per transport mode. */
  inheritsAssessmentByMode?: Partial<Record<Mode, string>>;
  /** Per-example display labels for node kinds. */
  nodeLabels?: Partial<Record<NodeKind, string>>;
}

export type DiagramItem = Scenario | Architecture;
