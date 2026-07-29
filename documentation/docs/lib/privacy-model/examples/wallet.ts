// Worked example: wallet block-sync (the Zcash / lightwalletd case).
//
// Instantiates the generic threat-model spine (../threat-model) with the
// application-specific parts that decision D1 demoted out of the spine:
//  - an L1 "chain-only observer" actor (public on-chain data);
//  - the invariants a wallet must hold (identity↔balance, tx grouping);
//  - the concrete scenarios, whose destination node renders as "lightwalletd".
//
// Content reconciled from Claudia Diaz's note and the original Zcash scenario
// doc (https://nymtech.atlassian.net/wiki/spaces/KB/pages/899907585/zcash).

import type { Invariant, MatrixCell, Scenario, ThreatActor, Tri } from "../types";

/** The wallet example's public/out-of-band observer (spine actors add L2/L3). */
export const WALLET_L1: ThreatActor = {
  id: "L1",
  name: "Chain-only observer",
  vantage: "Knows only what is publicly visible on the chain.",
  observes: ["Public on-chain data (including public migration amounts)"],
  cannotObserve: ["Anything off-chain"],
  cost: "Free — the public chain.",
};

export const WALLET_INVARIANTS: Invariant[] = [
  {
    id: "A",
    name: "Linkage between identity and approximate balance",
    statement:
      "The adversary cannot link a user identifier (e.g. client IP address) to a balance, even approximately. Since migration amounts are public, the invariant survives only as long as no migration transaction, and no group of them, is attributable to the user.",
    dependsOn:
      "P1 at every actor plus V2/V3 discipline — attribution can be direct (a broadcast from the user's home IP) or transitive (a broadcast linkable to attributable activity such as a sync session).",
  },
  {
    id: "B",
    name: "Linkage between transactions",
    statement:
      "The adversary must not be able to group the migration transactions of a single wallet. Grouped amounts sum to the starting balance and feed subset-sum linkage against known amounts.",
    dependsOn: "P2 plus V3 content discipline.",
  },
];

/** Terse constructor for a matrix cell. */
function cell(verdict: Tri, text: string): MatrixCell {
  return { verdict, text };
}

/** Every wallet scenario renders its destination node as "lightwalletd". */
const WALLET_LABELS = { destination: "lightwalletd" } as const;

export const WALLET_SCENARIOS: Scenario[] = [
  {
    id: "unprotected",
    kind: "scenario",
    title: "Unprotected on the open internet",
    shortTitle: "Unprotected",
    nodeLabels: WALLET_LABELS,
    summary:
      "The wallet talks to lightwalletd directly. Identity and contents arrive together — the baseline everything else improves on.",
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
          "Every request and every broadcast",
          "All request timing and content",
        ],
        cantSee: [],
        p1: "no",
        p2: "no",
        residual: [
          "Both invariants fail with no adversarial effort — this is the baseline network protection must improve on.",
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
    cons: [
      "Needs IP, timing and block-height obfuscation (request blocks out of order + buffer)",
    ],
    fit: ["Baseline only — offers no protection"],
  },
  {
    id: "mixnet-ipr",
    kind: "scenario",
    title: "Mixnet via exit proxy (IP packet router) — single IPR",
    shortTitle: "Mixnet · single IPR",
    nodeLabels: WALLET_LABELS,
    summary:
      "Fixed-size Sphinx packets, three mix layers with per-hop delays, Poisson sending with cover, then an exit-side IP packet router to a stock lightwalletd. Strong against network observers; against the destination a fixed IPR behaves like dVPN single-exit.",
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
      p1L2: cell("yes", "destination sees the exit/IPR IP, not the client"),
      p2L2: cell("no", "a fixed IPR behaves like dVPN single-exit at the destination — rotate the IPR per request"),
      p1L3L: cell("yes", "constant-size packets, Poisson rate, cover traffic"),
      p1L3G: cell("partial", "resists per-packet correlation; long bulk flows weaken it"),
    },
    performance: { fastSync: "no", note: "5-hop + mixing delays" },
    requires: "—  (single IPR)",
    actorAssessment: [
      {
        actor: "L2",
        sees: ["Exit gateway / IPR IP", "All requests and contents"],
        cantSee: ["Client IP"],
        p1: "yes",
        p2: "no",
        residual: [
          "The wallet's TCP connection to lightwalletd is an ordinary end-to-end connection arriving from the exit's IP. Per-packet unlinkability in transit does not translate into request unlinkability at the destination.",
          "With a fixed IPR, P2 behaves exactly like the dVPN single-exit case — rotate the IPR per request to restore it.",
        ],
      },
      {
        actor: "L3L",
        sees: [
          "Constant-size packets at a Poisson rate with cover traffic",
          "That the client uses the Nym mixnet",
        ],
        cantSee: ["Reliable destination, volume or activity"],
        p1: "yes",
        residual: [],
      },
      {
        actor: "L3G",
        sees: ["Per-packet timing (hampered by mixing delays and cover traffic)"],
        cantSee: [],
        p1: "partial",
        residual: [
          "The extent to which bulk transfers of many packets can be correlated over time is an open question. The mixnet is strongest for small, independent messages and weakest for bulk sync — one reason bulk sync does not belong on the mixnet (the other is throughput).",
        ],
      },
    ],
    pros: ["Client IP heavily obfuscated (timing + location) by the Nym network"],
    cons: ["Compact-block sync very slow (5-hop + mixing delays)", "Fixed IPR is a linking key at the destination"],
    fit: [
      "Client identity + IP heavily hidden from lightwalletd",
      "Block sync very slow",
    ],
  },
];

export function getWalletScenario(id: string): Scenario | undefined {
  return WALLET_SCENARIOS.find((s) => s.id === id);
}
