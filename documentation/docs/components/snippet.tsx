// `<Snippet id="..." />` on the docs site (D9 fallback, task 10.6): a server
// component that renders a diagram citation from its `.mmd` file at build time.
// A code citation needs the extracted records and is not rendered here yet, so
// it fails the build with its id rather than render nothing.

import fs from "node:fs";
import path from "node:path";
// The copy that next.config.js aliases for Turbopack; one module for both bundlers.
import { Mermaid } from "../mermaid-client.mjs";

// The build runs in documentation/docs, where the snippets are (.nym-codex/snippets).
const SNIPPETS = path.resolve(process.cwd(), ".nym-codex/snippets");
const REPO = "nym/";

export function Snippet({ id }: { id: string }) {
  if (!id.startsWith(REPO)) throw new Error(`<Snippet id="${id}">: not a snippet of this repository`);
  if (id.includes("#")) throw new Error(`<Snippet id="${id}">: code citations are not rendered on the docs site yet`);
  const file = path.join(SNIPPETS, `${id.slice(REPO.length)}.mmd`);
  if (!fs.existsSync(file)) throw new Error(`<Snippet id="${id}">: no diagram at ${file}`);
  return <Mermaid chart={fs.readFileSync(file, "utf-8")} />;
}
