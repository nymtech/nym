// The generic Nym threat model — the app-agnostic spine.
//
// Sourced from Claudia Diaz's note "Network privacy" (§2–§3, §5) and
// generalised (decision D1): the primary adversary is "the destination", not
// any one application's server. Worked examples (wallet, messaging, browsing)
// instantiate this spine and add their own L1 actor and invariants.

import type {
  Layer,
  Property,
  ThreatActor,
  Vector,
} from "./types";

export const ACTORS: ThreatActor[] = [
  {
    id: "L2",
    name: "The destination",
    vantage:
      "Is, or has compromised, the service the application connects to — the server on the other end of the session.",
    observes: [
      "The source IP of incoming connections",
      "The fine-grained arrival time of every request",
      "The complete contents of each request: which endpoints or resources are fetched, the parameters, the operation, and the payload",
    ],
    cannotObserve: ["Only what the client never sends it"],
    cost: "Cheap and privileged. To run or compromise a service needs no network-wide vantage — it is the one party the user must talk to directly.",
    primary: true,
  },
  {
    id: "L3L",
    name: "Local network observer",
    vantage:
      "On the client's access network (Wi-Fi, ISP). Knows the user's IP.",
    observes: [
      "Where the user's traffic goes, and its timing and volume",
      "The timing and type of activity, through traffic fingerprinting",
    ],
    cannotObserve: ["TLS-protected contents"],
    cost: "Whoever runs the access network or the ISP.",
  },
  {
    id: "L3G",
    name: "Global network observer",
    vantage:
      "Observes both ends of the communication and all intermediate hops at once.",
    observes: [
      "Flow correlation (the timing, volume and size of a sequence of packets) at the input and output of every hop, or end to end",
    ],
    cannotObserve: ["TLS-protected contents"],
    cost: "A powerful adversary with global visibility.",
  },
];

/** All levels may hold auxiliary information (§2). */
export const AUXILIARY_INFO =
  "Adversaries at every level can also hold auxiliary information: leaked account records, data from a compromised service, or activity that a previous slip already attributed to the user.";

export const PROPERTIES: Property[] = [
  {
    id: "P1",
    name: "Request-identity unlinkability",
    definition:
      "The adversary cannot attribute a given request to the user — for example, through the client IP address.",
  },
  {
    id: "P2",
    name: "Request-request unlinkability",
    definition:
      "The adversary cannot determine that two requests come from the same client. A request is any protocol interaction: a page load, an API call, a message send.",
  },
];

/**
 * The P1/P2 asymmetry and the pseudonymous-profile trap (§3.1). Foregrounded
 * because it explains every "given per-request rotation" caveat in the matrix.
 */
export const PROFILE_NOTE =
  "The two properties are asymmetric. If requests are attributable to an identity, they are thereby linkable to each other, so a P1 failure across requests implies a P2 failure; the converse does not hold. Most configurations land in an intermediate pseudonymous-profile state, and it is fragile: one attributed request anywhere in the profile retroactively attributes the whole profile.";

export const VECTORS: Vector[] = [
  {
    id: "V1",
    name: "Session state",
    consistsOf:
      "The source IP the destination observes, the TCP/TLS connection state, the tunnel session, and any application-layer session identifiers.",
    observableFrom: ["L2"],
    countermeasures: [
      { text: "Relaying — hides the client IP", layer: "transport", against: ["L2"] },
      {
        text: "Short-lived connections — a new connection per request",
        layer: "transport",
        against: ["L2"],
      },
      {
        text: "Exit rotation per connection (in dVPN and mixnet alike)",
        layer: "transport",
        against: ["L2"],
      },
    ],
  },
  {
    id: "V2",
    name: "Timing",
    consistsOf:
      "At the destination, the arrival times of requests. On network links, the timing, size and volume of packets.",
    observableFrom: ["L2", "L3L", "L3G"],
    countermeasures: [
      {
        text: "Client-side: randomised request times and request scheduling — against the destination",
        layer: "hygiene",
        against: ["L2"],
      },
      {
        text: "In transit: mixing delays, Poisson sending, cover traffic — against network observers",
        layer: "transport",
        against: ["L3L", "L3G"],
      },
    ],
  },
  {
    id: "V3",
    name: "Content",
    consistsOf:
      "The endpoints and resources requested, the parameters and query values, the operation performed, and the payload bytes.",
    observableFrom: ["L2"],
    countermeasures: [
      {
        text: "Request-shape discipline: padding, batching, and decoy or overlapping requests",
        layer: "hygiene",
        against: ["L2"],
      },
    ],
  },
];

/**
 * The category error to foreground (§3.2): in-transit mixing does not protect
 * against the destination. This is the flagship claim of the two-layer model.
 */
export const CATEGORY_ERROR =
  "A common category error credits in-transit mixing as protection against the destination (L2). Mixing delays and cover traffic change what a network observer can infer; the destination sees only what arrives, and when it arrives. No amount of mixing protects a user against the server they are talking to. What protects them is the absence of session state (V1), plus timing and content discipline (V2/V3) on the requests themselves.";

export interface LayerInfo {
  id: Layer;
  title: string;
  summary: string;
  provides: string[];
}

/** The two-layer model. */
export const LAYERS: LayerInfo[] = [
  {
    id: "transport",
    title: "Layer 1 — Transport",
    summary:
      "What the configuration chooses. Provides V1 identity protection, and V2 in-transit timing protection against network observers.",
    provides: [
      "V1 (identity) vs L2: relay + short-lived connections + exit rotation",
      "V2 (in-transit timing) vs L3L/L3G: mixing, Poisson sending, cover traffic (mixnet only)",
    ],
  },
  {
    id: "hygiene",
    title: "Layer 2 — Baseline hygiene",
    summary:
      "Transport-independent client discipline (§5), owed by every application regardless of which transport it chooses.",
    provides: [
      "V2 (timing) vs L2: randomised request times; requests decorrelated from activity milestones",
      "V3 (content) vs L2: request-shape discipline (padding, batching, decoy requests)",
    ],
  },
];
