// `snippet:<id>` links on the docs site (D9 fallback, task 10.6): each id becomes
// the route and anchor of the page that imports the target. The routes come
// from `.nym-codex/build/links-docs.json`, which lib/snippets/links.mts writes in
// `predev` and the builds, so the site bundles no tool code. A target that no
// docs page imports fails the build, as the spec requires ("A link names a
// snippet id").

import fs from "node:fs";
import path from "node:path";

// The build runs in documentation/docs.
const ROUTES = path.resolve(process.cwd(), ".nym-codex/build/links-docs.json");

// The routes and the modification time of the file they came from, so a rerun
// of links.mts shows on the next request in `next dev`.
let cache: { mtimeMs: number; routes: Record<string, string> } | null = null;

function snippetRoutes(): Record<string, string> {
  if (!fs.existsSync(ROUTES)) throw new Error(`${ROUTES} is missing; run node lib/snippets/links.mts --site docs --out .nym-codex/build/links-docs.json`);
  const { mtimeMs } = fs.statSync(ROUTES);
  if (cache?.mtimeMs !== mtimeMs) {
    const routes: Record<string, string> = JSON.parse(fs.readFileSync(ROUTES, "utf-8"));
    cache = { mtimeMs, routes };
  }
  return cache.routes;
}

/** The site route for a `snippet:` href; any other href is returned as it is. */
export function snippetHref(href: string | undefined): string | undefined {
  if (!href?.startsWith("snippet:")) return href;
  const id = href.slice("snippet:".length);
  const routes = snippetRoutes();
  if (!Object.hasOwn(routes, id)) throw new Error(`snippet:${id} is on no page of this site, or ${ROUTES} is older than the pages; rerun node lib/snippets/links.mts --site docs --out .nym-codex/build/links-docs.json`);
  return routes[id];
}
