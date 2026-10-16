// `<Snippet id="..." />` on the docs site (D9 fallback, tasks 10.6 and 6b.5): a
// server component that renders a citation at build time.
// - A diagram citation renders its `.mmd` file.
// - A threat-model citation reads the record by id from `.nym-codex/build/model.jsonl`
//   (`threat-model.mjs --out`, run by `predev` and `build`) and passes `meta.data`
//   to the view, so the page imports no model module. A scenario has the views
//   `diagram`, `assessment`, and `card`; a scenario variant (another drawing of a
//   scenario) has `diagram` only.
// A code citation needs the extracted records and is not rendered here yet, so
// it fails the build with its id rather than render nothing.

import fs from "node:fs";
import path from "node:path";
// The copy that next.config.js aliases for Turbopack; one module for both bundlers.
import { Mermaid } from "../mermaid-client.mjs";
import { ScenarioAssessment, ScenarioCard, ScenarioDiagram } from "./threat-model/views";
import type { Scenario } from "../lib/privacy-model/types";

// The build runs in documentation/docs, where the snippets are (.nym-codex/snippets).
const SNIPPETS = path.resolve(process.cwd(), ".nym-codex/snippets");
const MODEL = path.resolve(process.cwd(), ".nym-codex/build/model.jsonl");
const REPO = "nym/";

// The same views as MODEL_VIEWS and VARIANT_VIEWS in lib/snippets/repo.mjs (a test checks it).
const VARIANT_VIEWS = ["diagram"];
const VIEWS = {
  diagram: (data: Scenario) => <ScenarioDiagram item={data} />,
  assessment: (data: Scenario) => <ScenarioAssessment scenario={data} />,
  card: (data: Scenario) => <ScenarioCard scenario={data} />,
};
type View = keyof typeof VIEWS;

// Read on each call, so a rerun of threat-model.mjs shows on the next request in
// `next dev`. The file is small, and a page cites a few records at most.
function modelRecord(id: string): { meta: { data: Scenario } } {
  if (!fs.existsSync(MODEL)) throw new Error(`<Snippet id="${id}">: ${MODEL} is missing; run node lib/snippets/threat-model.mjs --out .nym-codex/build/model.jsonl`);
  const lines = fs.readFileSync(MODEL, "utf-8").split("\n").filter(Boolean);
  const record = lines.map((line) => JSON.parse(line)).find((r) => r.id === id);
  if (!record) throw new Error(`<Snippet id="${id}">: no threat-model record with this id`);
  return record;
}

export function Snippet({ id, view }: { id: string; view?: View }) {
  if (!id.startsWith(REPO)) throw new Error(`<Snippet id="${id}">: not a snippet of this repository`);
  if (id.includes("#")) throw new Error(`<Snippet id="${id}">: code citations are not rendered on the docs site yet`);
  const model = id.includes("/threat-model/");
  if (model && view === undefined) throw new Error(`<Snippet id="${id}">: a threat-model citation needs view="${Object.keys(VIEWS).join('" | "')}"`);
  if (view !== undefined) {
    if (!model) throw new Error(`<Snippet id="${id}" view="${view}">: only a threat-model scenario or variant has a view`);
    if (!Object.hasOwn(VIEWS, view)) throw new Error(`<Snippet id="${id}" view="${view}">: the view is not one of ${Object.keys(VIEWS).join(", ")}`);
    const variant = id.includes("/threat-model/variant/");
    if (!variant && !/\/threat-model\/(example\/[^/]+\/)?scenario\//.test(id)) throw new Error(`<Snippet id="${id}" view="${view}">: only a scenario or a scenario variant has views`);
    if (variant && !VARIANT_VIEWS.includes(view)) throw new Error(`<Snippet id="${id}" view="${view}">: a scenario variant has the view ${VARIANT_VIEWS.join(", ")} only`);
    return VIEWS[view](modelRecord(id).meta.data);
  }
  const file = path.join(SNIPPETS, `${id.slice(REPO.length)}.mmd`);
  if (!fs.existsSync(file)) throw new Error(`<Snippet id="${id}">: no diagram at ${file}`);
  return <Mermaid chart={fs.readFileSync(file, "utf-8")} />;
}
