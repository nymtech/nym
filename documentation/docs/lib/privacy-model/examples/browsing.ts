// Worked example: private web browsing against a website or API.
//
// Instantiates the generic threat-model spine (../threat-model) with the
// application-specific parts:
//  - an L1 "public-record observer" actor (public DNS and CT logs);
//  - the invariants a browser must hold (history not attributable, requests
//    not groupable into a profile);
//  - the concrete scenarios, whose destination node renders as "website".
//
// Dominant vector: V3 content. The URL and query are the request, so the
// website reads which pages you fetch even when your IP is hidden. Transport
// does not touch this; content discipline (the hygiene layer) does.

import type { Invariant, MatrixCell, Scenario, ThreatActor, Tri } from "../types";
import { multipathDvpnTopology } from "../topologies";

/** The browsing example's public observer (spine actors add L2/L3). */
export const BROWSING_L1: ThreatActor = {
  id: "L1",
  name: "Public-record observer",
  vantage: "Reads public DNS and Certificate Transparency logs.",
  observes: [
    "Which domains and subdomains exist and resolve",
    "TLS certificates issued for those names",
  ],
  cannotObserve: [
    "Which user visits a site",
    "Which specific pages, URLs or queries are requested",
    "Request content",
  ],
  cost: "Free: public DNS and CT logs.",
};

export const BROWSING_INVARIANTS: Invariant[] = [
  {
    id: "A",
    name: "Browsing history not attributable to a person",
    statement:
      "The adversary cannot link the set of pages, URLs and queries requested to a real identity, even approximately.",
    dependsOn:
      "P1 (hide the client IP) plus P2 (rotate exits per request) plus V3 content discipline. The URLs and queries name the content, so IP-hiding alone leaves the history fully readable by the site.",
  },
  {
    id: "B",
    name: "Requests not groupable into a profile",
    statement:
      "The adversary must not group a user's requests into one browsing profile. Grouped requests rebuild the history the first invariant protects.",
    dependsOn:
      "P2 (rotate exits per request) plus V3 request-shape discipline. A fixed exit, a session cookie or a stable request fingerprint re-groups the requests.",
  },
];

/** Terse constructor for a matrix cell. */
function cell(verdict: Tri, text: string): MatrixCell {
  return { verdict, text };
}

/** Every browsing scenario renders its destination node as "website". */
const BROWSING_LABELS = { destination: "website" } as const;

export const BROWSING_SCENARIOS: Scenario[] = [
  {
    id: "unprotected",
    kind: "scenario",
    title: "Unprotected on the open internet",
    shortTitle: "Unprotected",
    nodeLabels: BROWSING_LABELS,
    summary:
      "The browser talks to the website directly. The client IP, every URL and query, and the full request pattern arrive together.",
    paths: [
      {
        id: "direct",
        mode: "direct",
        stages: ["client", "internet", "destination"],
      },
    ],
    matrix: {
      p1L2: cell("no", "client IP and requested URLs arrive together"),
      p2L2: cell("no", "requests trivially grouped by client IP"),
      p1L3L: cell("no", "endpoints, timing and volume visible"),
      p1L3G: cell("no", "L3L and L3G collapse into one observer"),
    },
    performance: { fastSync: "yes" },
    requires: "None",
    actorAssessment: [
      {
        actor: "L2",
        sees: [
          "Real client IP",
          "Every URL and query string",
          "The full request pattern and content",
        ],
        cantSee: [],
        p1: "no",
        p2: "no",
        residual: [
          "The browsing history is attributed with no adversarial effort. This is the baseline network protection must improve on.",
        ],
      },
      {
        actor: "L3L",
        sees: ["Endpoints, timing and volume of the page loads"],
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
    cons: ["Needs IP hiding and exit rotation, then content discipline on the requests themselves"],
    fit: ["Baseline only: offers no protection"],
  },
  {
    id: "dvpn-multi",
    kind: "scenario",
    title: "dVPN to the website, multiple exits",
    shortTitle: "dVPN · multi-exit",
    nodeLabels: BROWSING_LABELS,
    topology: multipathDvpnTopology(3),
    summary:
      "Distinct WireGuard peers at several exits split requests across exit IPs. Per-request rotation unlinks requests at the transport layer, and browsing stays fast. The website still reads the content of every request.",
    paths: [
      { id: "dvpn-a", label: "Exit A", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] },
      { id: "dvpn-b", label: "Exit B", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] },
      { id: "dvpn-c", label: "Exit C", mode: "dvpn", stages: ["client", "entry", "exit", "destination"] },
    ],
    matrix: {
      p1L2: cell("yes", "the website sees exit IPs, not the client"),
      p2L2: cell("yes", "unlinkable at the transport layer given per-request rotation; a logged-in session or personalised query re-links at the application layer"),
      p1L3L: cell("no", "no mixing: page-load timing and volume fingerprint the site"),
      p1L3G: cell("no", "end-to-end flow correlation"),
    },
    performance: { fastSync: "yes" },
    requires: "automate exit rotation per request",
    actorAssessment: [
      {
        actor: "L2",
        sees: [
          "Requests arriving from many exit gateways",
          "The exact URL, query string and body of every request",
        ],
        cantSee: [
          "Client IP",
          "A complete per-client profile, for anonymous fetches split across exits",
        ],
        p1: "yes",
        p2: "yes",
        residual: [
          "IP-hiding and exit rotation do not touch the dominant vector. The website reads which pages you request, because the URL and query are the request.",
          "Any request that carries an identifier (a login cookie, a personalised query) re-links the session regardless of exit. Content discipline is a hygiene-layer job that transport cannot do.",
        ],
      },
      {
        actor: "L3L",
        sees: [
          "Activity fingerprint: dVPN adds no cover and preserves packet timing",
          "Page-load bursts that fingerprint which site is visited",
        ],
        cantSee: [],
        residual: [],
      },
      {
        actor: "L3G",
        sees: ["End-to-end flow correlation"],
        cantSee: [],
        residual: [
          "The 2-hop route stops a single gateway linking client to website, but colluding gateways or a global observer still correlate.",
        ],
      },
    ],
    pros: [
      "The website sees traffic from many exit gateways, not the client",
      "Fast dVPN speeds suit bulk page loads",
    ],
    cons: [
      "The website still reads every URL and query you request",
      "No timing protection against network observers",
    ],
    fit: [
      "Hides who you are from the website, not what you fetch",
      "Pair with content discipline to protect the history itself",
    ],
  },
];

export function getBrowsingScenario(id: string): Scenario | undefined {
  return BROWSING_SCENARIOS.find((s) => s.id === id);
}

export function requireBrowsingScenario(id: string): Scenario {
  const s = getBrowsingScenario(id);
  if (!s) throw new Error(`unknown browsing scenario: ${id}`);
  return s;
}
