// Generic (application-agnostic) scenarios for the abstract spine pages — the
// two-layer model, the actors/vectors/properties references, and the decision
// tool. The destination is an unlabelled "destination" node and the prose names
// no application. Worked examples (wallet, messaging, browsing) live in their
// own files and add application-specific labels and invariants.

import type { MatrixCell, Scenario, Tri } from "../types";
import {
  hybridTopology,
  multipathDvpnTopology,
  multipathMixnetTopology,
} from "../topologies";

/** Terse constructor for a matrix cell. */
function cell(verdict: Tri, text: string): MatrixCell {
  return { verdict, text };
}

export const GENERIC_SCENARIOS: Scenario[] = [
  {
    id: "unprotected",
    kind: "scenario",
    title: "Unprotected on the open internet",
    shortTitle: "Unprotected",
    summary:
      "The client talks to the destination directly. Identity and contents arrive together — the baseline everything else improves on.",
    paths: [
      {
        id: "direct",
        mode: "direct",
        stages: ["client", "internet", "destination"],
      },
    ],
    matrix: {
      p1L2: cell("no", "identity and contents arrive together"),
      p2L2: cell("no", "requests trivially grouped by client IP"),
      p1L3L: cell("no", "endpoints, timing and volume visible"),
      p1L3G: cell("no", "L3L and L3G collapse into one observer"),
    },
    performance: { fastSync: "yes" },
    requires: "—",
    actorAssessment: [
      {
        actor: "L2",
        sees: [
          "Real client IP",
          "Every request",
          "All request timing and content",
        ],
        cantSee: [],
        p1: "no",
        p2: "no",
        residual: [
          "Everything is exposed with no adversarial effort — this is the baseline network protection must improve on.",
        ],
      },
      {
        actor: "L3L",
        sees: ["Endpoints, timing and volume of the exchanged flows"],
        cantSee: [],
        residual: [],
      },
      {
        actor: "L3G",
        sees: ["Endpoints, timing and volume (collapses with L3L on a direct connection)"],
        cantSee: [],
        residual: [],
      },
    ],
    pros: [],
    cons: ["Needs IP hiding, then timing and content discipline"],
    fit: ["Baseline only — offers no protection"],
  },
  {
    id: "vpn",
    kind: "scenario",
    title: "Centralised VPN",
    shortTitle: "VPN",
    summary:
      "A single VPN server hides the client IP behind its exit IP and nothing else. Within a session requests stay grouped by connection state, and the operator sees both ends.",
    paths: [{ id: "vpn", mode: "vpn", stages: ["client", "vpn", "destination"] }],
    matrix: {
      p1L2: cell("yes", "hides the client IP; the destination sees the VPN's exit IP"),
      p2L2: cell("no", "connection state groups requests; the exit IP re-identifies across sessions"),
      p1L3L: cell("no", "no in-transit timing protection"),
      p1L3G: cell("no", "the operator sees both ends — effectively a global observer"),
    },
    performance: { fastSync: "yes" },
    requires: "—",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["VPN exit IP", "All requests and contents"],
        cantSee: ["Client IP"],
        p1: "yes",
        p2: "no",
        residual: [
          "Within a session requests stay grouped by connection state; across sessions the exit IP re-identifies the client.",
        ],
      },
      { actor: "L3L", sees: ["Activity fingerprint (no in-transit protection)"], cantSee: [], residual: [] },
      { actor: "L3G", sees: ["Both ends — a single operator can act as a global observer"], cantSee: [], residual: [] },
    ],
    pros: ["Fast"],
    cons: ["A single operator sees both ends", "Add timing and content discipline"],
    fit: ["Fast IP hiding, but weak unlinkability and a single trusted operator"],
  },
  {
    id: "dvpn-single",
    kind: "scenario",
    title: "dVPN — 2-hop, single exit",
    shortTitle: "dVPN · single exit",
    summary:
      "A 2-hop WireGuard tunnel (entry + exit gateway) hides the client IP. A single fixed exit leaves requests linkable at the destination within a session and weak across sessions.",
    paths: [{ id: "dvpn", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] }],
    matrix: {
      p1L2: cell("yes", "the destination sees the exit gateway's IP, not the client"),
      p2L2: cell("no", "one NATed flow within a session; weak across sessions (exit crowding)"),
      p1L3L: cell("no", "activity fingerprintable"),
      p1L3G: cell("no", "end-to-end flow correlation"),
    },
    performance: { fastSync: "yes" },
    requires: "nothing special",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["Exit gateway IP", "All requests and contents"],
        cantSee: ["Client IP"],
        p1: "yes",
        p2: "no",
        residual: [
          "P2 fails within a session: the tunnel delivers one NATed flow. Across sessions, linkage depends on crowding at the exit IP.",
          "Rotate the exit per request to restore P2 — see the multi-exit configuration.",
        ],
      },
      { actor: "L3L", sees: ["Activity fingerprint — WireGuard adds no cover and preserves packet timing"], cantSee: [], residual: [] },
      {
        actor: "L3G",
        sees: ["End-to-end flow correlation"],
        cantSee: [],
        residual: [
          "The 2-hop route stops a single gateway linking client to destination, but both gateways colluding, or a global observer, can still correlate.",
        ],
      },
    ],
    pros: ["Fast", "Deployable today"],
    cons: ["A fixed exit is a linking key — rotate per request"],
    fit: ["The minimum: hides IP; add exit rotation and baseline hygiene"],
  },
  {
    id: "dvpn-multi",
    kind: "scenario",
    title: "dVPN — 2-hop, multiple exits",
    shortTitle: "dVPN · multi-exit",
    topology: multipathDvpnTopology(3),
    summary:
      "Distinct WireGuard peers registered at several exits split traffic across exit IPs. Given per-request rotation this restores request-request unlinkability at the destination. Fast, but no mixing, so network-observer exposure is unchanged.",
    paths: [
      { id: "dvpn-a", label: "Exit A", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] },
      { id: "dvpn-b", label: "Exit B", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] },
      { id: "dvpn-c", label: "Exit C", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] },
    ],
    matrix: {
      p1L2: cell("yes", "the destination sees exit IPs, not the client"),
      p2L2: cell("yes", "given per-request exit rotation and baseline hygiene"),
      p1L3L: cell("no", "as single-exit: activity fingerprintable"),
      p1L3G: cell("no", "as single-exit: end-to-end correlation"),
    },
    performance: { fastSync: "yes" },
    requires: "automate exit rotation per request",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["Requests from many exit gateways"],
        cantSee: ["Client IP", "A complete per-client profile (requests split across exits)"],
        p1: "yes",
        p2: "yes",
        residual: [
          "P2 holds given per-request exit rotation plus baseline hygiene. The anonymity set is all clients reaching that destination via Nym.",
        ],
      },
      { actor: "L3L", sees: ["Activity fingerprint (same as single-exit — no cover, timing preserved)"], cantSee: [], residual: [] },
      { actor: "L3G", sees: ["End-to-end flow correlation (same as single-exit)"], cantSee: [], residual: [] },
    ],
    pros: ["The destination sees traffic from all exit gateways, not the client", "Fast dVPN speeds"],
    cons: ["Only useful when many users share the same exit set", "No timing protection against network observers"],
    fit: ["Fast IP hiding with request unlinkability; pair with mixnet for timing safety"],
  },
  {
    id: "mixnet",
    kind: "scenario",
    title: "Mixnet to a clearnet destination — single exit",
    shortTitle: "Mixnet · single exit",
    summary:
      "Fixed-size Sphinx packets take three mix layers with per-hop delays, Poisson sending, and cover traffic. The exit gateway then forwards to the destination over the public internet. Strong against network observers; against the destination a fixed exit behaves like a single dVPN exit.",
    paths: [
      {
        id: "mix",
        mode: "mixnet",
        stages: [
          "client",
          "entry",
          "mix",
          "mix",
          "mix",
          "exit",
          "destination",
        ],
      },
    ],
    matrix: {
      p1L2: cell("yes", "destination sees the exit gateway's IP, not the client"),
      p2L2: cell("no", "a fixed exit behaves like a single dVPN exit at the destination — rotate the exit per request"),
      p1L3L: cell("yes", "constant-size packets, Poisson rate, cover traffic"),
      p1L3G: cell("partial", "resists per-packet correlation; long bulk flows weaken it"),
    },
    performance: { fastSync: "no", note: "5-hop + mixing delays" },
    requires: "— (single exit)",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["Exit gateway IP", "All requests and contents"],
        cantSee: ["Client IP"],
        p1: "yes",
        p2: "no",
        residual: [
          "The client's connection to the destination is an ordinary end-to-end connection arriving from the exit gateway's IP. Per-packet unlinkability in transit does not become request unlinkability at the destination.",
          "With a fixed exit, P2 behaves like the single dVPN exit case. Rotate the exit per request to restore it.",
        ],
      },
      {
        actor: "L3L",
        sees: [
          "Constant-size packets at a Poisson rate with cover traffic",
          "That the client uses the Nym mixnet",
        ],
        cantSee: ["The destination, the volume, or the activity"],
        p1: "yes",
        residual: [],
      },
      {
        actor: "L3G",
        sees: ["Per-packet timing (hampered by mixing delays and cover traffic)"],
        cantSee: [],
        p1: "partial",
        residual: [
          "How far bulk transfers of many packets can be correlated over time is an open question. The mixnet is strongest for small, independent messages and weakest for bulk transfers.",
        ],
      },
    ],
    pros: ["Client IP heavily obfuscated (timing + location) by the Nym network"],
    cons: ["Slow (5-hop + mixing delays)", "A fixed exit gateway is a linking key at the destination"],
    fit: [
      "Client identity and IP heavily hidden from the destination",
      "Not for bulk or latency-sensitive transfers",
    ],
  },
  {
    id: "mixnet-rotating",
    kind: "scenario",
    title: "Mixnet — rotating exits",
    shortTitle: "Mixnet · rotating exit",
    topology: multipathMixnetTopology(3),
    summary:
      "The same mixnet path, but requests are spread across several exit gateways. Rotating the exit per request restores unlinkability at the destination — the mixnet analogue of multi-exit dVPN. Still slow.",
    paths: [
      { id: "mix-a", label: "Exit A", mode: "mixnet", stages: ["client", "entry", "mix", "mix", "mix", "exit", "destination"] },
      { id: "mix-b", label: "Exit B", mode: "mixnet", stages: ["client", "entry", "mix", "mix", "mix", "exit", "destination"] },
      { id: "mix-c", label: "Exit C", mode: "mixnet", stages: ["client", "entry", "mix", "mix", "mix", "exit", "destination"] },
    ],
    matrix: {
      p1L2: cell("yes", "the destination sees exit IPs, not the client"),
      p2L2: cell("yes", "given per-request exit rotation"),
      p1L3L: cell("yes", "as single exit: constant-size, Poisson, cover"),
      p1L3G: cell("partial", "as single exit: resists per-packet; long flows weaken it"),
    },
    performance: { fastSync: "no", note: "5-hop + mixing delays" },
    requires: "ensure exit rotation per request",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["Requests from many exit gateways, with timing delays"],
        cantSee: ["Client IP", "A complete per-client profile"],
        p1: "yes",
        p2: "yes",
        residual: [
          "Fixed exit behaves like dVPN single-exit; rotating exit restores P2. Intersection attacks over long-running sessions remain, and few users per exit shrinks the anonymity set.",
        ],
      },
      { actor: "L3L", sees: ["Constant-size packets at a Poisson rate with cover traffic"], cantSee: ["The destination, the volume, or the activity"], p1: "yes", residual: [] },
      {
        actor: "L3G",
        sees: ["Per-packet timing (hampered by mixing and cover)"],
        cantSee: [],
        p1: "partial",
        residual: ["Long bulk flows weaken per-packet correlation resistance (open question)."],
      },
    ],
    pros: ["The destination sees traffic from all Nym exit gateways, not the client"],
    cons: ["Slow (5-hop + mixing delays)"],
    fit: ["Client identity and IP heavily hidden; not for bulk transfers"],
  },
  {
    id: "hybrid",
    kind: "scenario",
    title: "Hybrid — dVPN for bulk, mixnet for sensitive",
    shortTitle: "Hybrid",
    recommended: true,
    topology: hybridTopology({
      mixReplies: true,
      caption:
        "One client and entry gateway. Bulk traffic takes a dVPN route to its exit gateway; sensitive requests take the mixnet to a separate exit gateway. Both reach the destination, but they share no session state.",
    }),
    summary:
      "The practical target: bulk, latency-sensitive traffic over fast dVPN (multi-exit), and small timing-sensitive requests over the mixnet. The two streams share no session state, so the sensitive requests are unlinkable to the bulk profile by default.",
    paths: [
      { id: "bulk", label: "Bulk", mode: "dvpn", stages: ["client", "entry", "exit", "destination"], note: "Bandwidth-bound, not timing-sensitive → dVPN (fast)." },
      { id: "sensitive", label: "Sensitive", mode: "mixnet", stages: ["client", "entry", "mix", "mix", "mix", "exit", "destination"], note: "Small and timing-sensitive → mixnet (mixing, cover, location privacy)." },
    ],
    matrix: {
      p1L2: cell("yes", "the bulk tunnel hides the client IP"),
      p2L2: cell("yes", "sensitive requests unlinkable to bulk and to each other (per-request rotation)"),
      p1L3L: cell("partial", "bulk activity fingerprintable; sensitive requests unobservable"),
      p1L3G: cell("partial", "bulk correlatable but uninformative; sensitive requests protected"),
    },
    performance: { fastSync: "yes", note: "fast bulk (dVPN) / slow but timing-safe sensitive (mixnet)" },
    requires: "both dVPN and mixnet stacks in one process",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["Bulk side: a pseudonymous profile", "Sensitive side: arrival timing on the mixnet path"],
        cantSee: ["Client IP", "Which sensitive request belongs to which bulk session"],
        p1: "yes",
        p2: "yes",
        residual: [
          "The two streams share no session state (different transports, different IPs). Sensitive requests are unlinkable to the bulk profile and to each other via per-request rotation and decorrelated timing.",
        ],
      },
      { actor: "L3L", sees: ["That the client uses the bulk transport (fingerprintable)"], cantSee: ["Sensitive requests (routed over the mixnet — unobservable)"], p1: "partial", residual: [] },
      {
        actor: "L3G",
        sees: ["The bulk flow (correlatable)"],
        cantSee: ["Sensitive requests"],
        p1: "partial",
        residual: ["Bulk is correlatable but uninformative; the hybrid makes bulk uninformative rather than invisible."],
      },
    ],
    pros: ["Client IP hidden, fast bulk transfer, and timing-obfuscated sensitive requests in one design"],
    cons: ["Needs a per-request-class transport split and coordination on shared exit sets"],
    fit: ["The recommended architecture: fast private bulk + timing-safe sensitive requests"],
  },
  {
    id: "end-to-end",
    kind: "scenario",
    title: "End to end — both ends run Nym",
    shortTitle: "End to end",
    summary:
      "Both ends run Nym. Traffic never leaves the mixnet and there is no exit gateway or clearnet destination. The peer is reached end to end and never learns your IP; replies return through the mixnet via SURBs.",
    paths: [
      {
        id: "e2e",
        mode: "mixnet",
        stages: ["client", "entry", "mix", "mix", "mix", "service-provider"],
      },
    ],
    topology: {
      nodes: [
        { id: "c", kind: "client", col: 0, label: "Your client" },
        { id: "e", kind: "entry", col: 1 },
        { id: "m1", kind: "mix", col: 2 },
        { id: "m2", kind: "mix", col: 3 },
        { id: "m3", kind: "mix", col: 4 },
        { id: "pg", kind: "entry", col: 5, label: "Peer's gateway" },
        { id: "p", kind: "client", col: 6, label: "Nym peer" },
      ],
      routes: [
        { id: "e2e", mode: "mixnet", nodeIds: ["c", "e", "m1", "m2", "m3", "pg", "p"] },
      ],
      caption:
        "Both ends run Nym. The peer is reached end to end through its own gateway; there is no exit to the clearnet, and the peer never learns your IP. Replies return via SURBs.",
    },
    matrix: {
      p1L2: cell("yes", "no untrusted destination; the Nym peer never learns your IP"),
      p2L2: cell("yes", "no untrusted destination to link requests"),
      p1L3L: cell("yes", "constant-size packets, Poisson rate, cover traffic"),
      p1L3G: cell("partial", "resists per-packet correlation; long flows weaken it"),
    },
    performance: { fastSync: "no", note: "5-hop + mixing delays" },
    requires: "both ends run Nym",
    actorAssessment: [
      {
        actor: "L3L",
        sees: ["Constant-size packets at a Poisson rate with cover traffic"],
        cantSee: ["The peer, the volume, or the activity"],
        p1: "yes",
        residual: [],
      },
      {
        actor: "L3G",
        sees: ["Per-packet timing (hampered by mixing and cover)"],
        cantSee: [],
        p1: "partial",
        residual: ["Long bulk flows weaken per-packet correlation resistance (open question)."],
      },
    ],
    pros: ["No untrusted destination exists, so the L2 adversary is absent"],
    cons: ["Both ends must run Nym", "Slow (5-hop + mixing delays)"],
    fit: ["The strongest position: no untrusted destination at all"],
  },
];

export function getGenericScenario(id: string): Scenario | undefined {
  return GENERIC_SCENARIOS.find((s) => s.id === id);
}
