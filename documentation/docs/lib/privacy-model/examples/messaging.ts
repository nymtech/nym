// Worked example: private messaging against a centralised chat server.
//
// Instantiates the generic threat-model spine (../threat-model) with the
// application-specific parts:
//  - an L1 "directory observer" actor (a public contact directory);
//  - the invariants a messaging app must hold (social graph and conversation
//    timing unlinkability);
//  - the concrete scenarios, whose destination node renders as "chat server".
//
// Dominant vector: V2 timing / who-talks-to-whom. The chat server routes by
// authenticated account, so it reconstructs the social graph regardless of
// transport. Hiding the client IP does not hide the account graph.

import type { Invariant, MatrixCell, Scenario, ThreatActor, Tri } from "../types";

/** The messaging example's public observer (spine actors add L2/L3). */
export const MESSAGING_L1: ThreatActor = {
  id: "L1",
  name: "Directory observer",
  vantage: "Reads a public contact directory (registered handles, public group listings).",
  observes: [
    "Public account handles that exist",
    "Publicly listed group memberships",
  ],
  cannotObserve: ["Message content", "Who messages whom", "Message timing"],
  cost: "Free: the public directory.",
};

export const MESSAGING_INVARIANTS: Invariant[] = [
  {
    id: "A",
    name: "Social graph unlinkability",
    statement:
      "The adversary cannot reconstruct who communicates with whom. A single conversation pair, or a set of them, must not be attributable to two accounts.",
    dependsOn:
      "P1 at every actor is necessary but nowhere near sufficient. A server that authenticates accounts and routes each message to a named recipient learns the graph regardless of transport. Only end-to-end delivery or a metadata-private protocol removes the leak.",
  },
  {
    id: "B",
    name: "Conversation timing unlinkability",
    statement:
      "The adversary must not infer that two accounts are in a live conversation from message arrival timing. Request and response bursts otherwise pair the accounts.",
    dependsOn:
      "V2 timing discipline (cover traffic, decorrelated send times) defeats this at the network layer. Against the server, only metadata-private delivery removes it.",
  },
];

/** Terse constructor for a matrix cell. */
function cell(verdict: Tri, text: string): MatrixCell {
  return { verdict, text };
}

/** Every messaging scenario renders its destination node as "chat server". */
const MESSAGING_LABELS = { destination: "chat server" } as const;

export const MESSAGING_SCENARIOS: Scenario[] = [
  {
    id: "unprotected",
    kind: "scenario",
    title: "Unprotected on the open internet",
    shortTitle: "Unprotected",
    nodeLabels: MESSAGING_LABELS,
    summary:
      "The client talks to the chat server directly. Identity, recipient and timing arrive together, and a network observer pairs the two conversing clients by timing.",
    paths: [
      {
        id: "direct",
        mode: "direct",
        stages: ["client", "internet", "destination"],
      },
    ],
    matrix: {
      p1L2: cell("no", "client IP and account arrive together"),
      p2L2: cell("no", "the server routes by account, so every recipient and timestamp is grouped by sender"),
      p1L3L: cell("no", "your request and response rhythm reveals a live conversation"),
      p1L3G: cell("no", "sees both endpoints and pairs the two conversing clients by timing"),
    },
    performance: { fastSync: "yes" },
    requires: "None",
    actorAssessment: [
      {
        actor: "L2",
        sees: [
          "Real client IP",
          "Your account and every message recipient",
          "The arrival time of every message",
        ],
        cantSee: [],
        p1: "no",
        p2: "no",
        residual: [
          "The social graph is reconstructed with no adversarial effort. This is the baseline network protection must improve on.",
        ],
      },
      {
        actor: "L3L",
        sees: [
          "That you are connected to the chat server",
          "Your request and response rhythm, which reveals a live conversation",
        ],
        cantSee: ["Which other user you are conversing with (pairing two clients needs both endpoints)"],
        residual: [],
      },
      {
        actor: "L3G",
        sees: ["Both endpoints, so it pairs the two conversing clients by correlated timing"],
        cantSee: [],
        residual: [],
      },
    ],
    pros: [],
    cons: ["Needs IP hiding, then network-layer timing protection and metadata-private delivery"],
    fit: ["Baseline only: offers no protection"],
  },
  {
    id: "mixnet",
    kind: "scenario",
    title: "Mixnet to the chat server, single exit",
    shortTitle: "Mixnet · single exit",
    nodeLabels: MESSAGING_LABELS,
    summary:
      "Fixed-size Sphinx packets take three mix layers with per-hop delays, Poisson sending and cover traffic. The exit gateway forwards to the chat server. Mixing hides the conversation from network observers, but the server still routes by account.",
    paths: [
      {
        id: "mix",
        mode: "mixnet",
        stages: ["client", "entry", "mix", "mix", "mix", "exit", "destination"],
      },
    ],
    matrix: {
      p1L2: cell("yes", "the server sees the exit gateway IP, not the client"),
      p2L2: cell("no", "the server authenticates your account and routes each message to a named recipient, so the social graph survives"),
      p1L3L: cell("yes", "constant-size packets, Poisson rate and cover traffic hide even that you are conversing"),
      p1L3G: cell("partial", "mixing degrades the timing that pairs the two clients; long conversations weaken it"),
    },
    performance: { fastSync: "no", note: "5-hop and mixing delays" },
    requires: "None (single exit)",
    actorAssessment: [
      {
        actor: "L2",
        sees: [
          "Exit gateway IP",
          "Your account and every message recipient",
          "The arrival time of every message",
        ],
        cantSee: ["Client IP"],
        p1: "yes",
        p2: "no",
        residual: [
          "Hiding your IP removes one identifier. The server still authenticates your account and delivers each message to a named recipient, so it reconstructs the social graph from account metadata regardless of transport.",
          "Message and reply timing at the server still reveals a live conversation. Only end-to-end delivery (no untrusted server) or a metadata-private protocol removes this leak.",
        ],
      },
      {
        actor: "L3L",
        sees: [
          "Constant-size packets at a Poisson rate with cover traffic",
          "That the client uses the Nym mixnet",
        ],
        cantSee: ["Whether you are in a conversation, the volume, or the activity"],
        p1: "yes",
        residual: [
          "Cover traffic hides even that you are conversing from a local observer. This is the layer the mixnet actually protects for messaging.",
        ],
      },
      {
        actor: "L3G",
        sees: ["Per-packet timing across both endpoints (hampered by mixing delays and cover traffic)"],
        cantSee: [],
        p1: "partial",
        residual: [
          "Mixing degrades the timing correlation that pairs the two conversing clients, but long-running conversations of many messages weaken that resistance (an open question).",
        ],
      },
    ],
    pros: ["Client IP hidden; mixing hides the conversation from a local observer and degrades global timing correlation"],
    cons: [
      "The chat server still learns who you message and when",
      "Slow (5-hop and mixing delays)",
    ],
    fit: [
      "Hides the conversation from the network, not from the server",
      "For server-side metadata privacy, run end-to-end instead",
    ],
  },
];

export function getMessagingScenario(id: string): Scenario | undefined {
  return MESSAGING_SCENARIOS.find((s) => s.id === id);
}

export function requireMessagingScenario(id: string): Scenario {
  const s = getMessagingScenario(id);
  if (!s) throw new Error(`unknown messaging scenario: ${id}`);
  return s;
}
