// Centralised hrefs into the threat-model pages. The #actor-Ln anchors are the
// stable cross-reference spine — the diagrams and assessment panels link to
// them — so the anchor ids never change; only this base path does.

import type { ActorId } from "./types";

const ACTORS_PAGE = "/network/threat-model/actors";

/** Link to a specific actor (L1/L2/L3L/L3G) on the actors page. */
export function actorHref(actor: ActorId): string {
  return `${ACTORS_PAGE}#actor-${actor}`;
}
