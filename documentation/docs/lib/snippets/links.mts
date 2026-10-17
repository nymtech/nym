// The snippet routes of one site as a JSON file: snippet id -> the route and
// anchor that a `snippet:` link to it becomes. The docs site reads the file
// (components/snippet-link.ts), so the site bundles no tool code; `predev` and
// the builds write it after the threat-model step. push.mts gives codex the
// link map of its own site as pages.json.
//
//   node lib/snippets/links.mts --site docs --out .nym-codex/build/links-docs.json

import fs from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';
import type { Repo } from './repo.mts';
import { openRepo, pageLinks, linkRoute } from './repo.mts';

/** Each snippet id that a page of `site` imports, to its route, sorted by id. */
export function snippetRoutes(repo: Repo, site: string): Record<string, string> {
  const map = pageLinks(repo, site);
  return Object.fromEntries([...map.keys()].sort().map((id) => [id, linkRoute(map, id)]));
}

function main(argv: string[]): void {
  const { site, out } = parseArgs({ args: argv, options: { site: { type: 'string' }, out: { type: 'string' } } }).values;
  if (!site || !out) throw new Error('usage: links.mts --site <site> --out <file>');
  const routes = snippetRoutes(openRepo(), site);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, `${JSON.stringify(routes, null, 2)}\n`);
  console.log(`wrote ${out}: ${Object.keys(routes).length} snippet routes of the site "${site}"`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
