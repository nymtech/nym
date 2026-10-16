// `snippet:<id>` links on the docs site (D9 fallback, task 10.6): the link map
// of the docs site turns each id into the route and anchor of the page that
// imports the target. A target that no docs page imports fails the build, as
// the spec requires ("A link names a snippet id").

import path from "node:path";
import { openRepo, pageLinks, linkRoute } from "../lib/snippets/repo.mjs";

let links: Map<string, { page: string; anchor: string | null }> | null = null;

/** The site route for a `snippet:` href; any other href is returned as it is. */
export function snippetHref(href: string | undefined): string | undefined {
  if (!href?.startsWith("snippet:")) return href;
  // The build runs in documentation/docs; the repository root is two levels up.
  links ??= pageLinks(openRepo(path.resolve(process.cwd(), "../..")), "docs");
  return linkRoute(links, href.slice("snippet:".length));
}
