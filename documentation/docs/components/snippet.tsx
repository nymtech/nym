// `<Snippet id="..." />` on the docs site (D9 fallback, tasks 10.6 and 6b.5): a
// server component that renders a citation at build time.
// - A diagram citation renders its `.mmd` file.
// - A citation of a whole kind (`nym/threat-model/actor` with `view="reference"`,
//   `nym/threat-model/scenario` with `view="matrix"`) renders the table of its
//   generic records, in the order of the source data (`meta.order`).
// - A threat-model citation reads the record by id from `.nym-codex/build/model.jsonl`
//   (`threat-model.mts --out`, run by `predev` and `build`) and passes `meta.data`
//   to the view, so the page imports no model module. A scenario has the views
//   `diagram`, `assessment`, and `card`; a scenario variant (another drawing of a
//   scenario) has `diagram` only.
// A code citation needs the extracted records and is not rendered here yet, so
// it fails the build with its id rather than render nothing.

import fs from "node:fs";
import path from "node:path";
// The copy that next.config.js aliases for Turbopack; one module for both bundlers.
import { Mermaid } from "../mermaid-client.mjs";
import { ActorsTable, PropertiesTable, ScenarioAssessment, ScenarioCard, ScenarioDiagram, ScenarioMatrix, VectorsTable } from "./threat-model/views";
import type { Property, Scenario, ThreatActor, Vector } from "../lib/privacy-model/types";

// The build runs in documentation/docs, where the snippets are (.nym-codex/snippets).
const SNIPPETS = path.resolve(process.cwd(), ".nym-codex/snippets");
const MODEL = path.resolve(process.cwd(), ".nym-codex/build/model.jsonl");
const REPO = "nym/";

// The same views as MODEL_VIEWS, VARIANT_VIEWS, and COLLECTION_VIEWS in lib/snippets/repo.mts (a test checks it).
const VARIANT_VIEWS = ["diagram"];
const COLLECTIONS = {
  actor: { view: "reference", render: (data: unknown[]) => <ActorsTable actors={data as ThreatActor[]} /> },
  vector: { view: "reference", render: (data: unknown[]) => <VectorsTable vectors={data as Vector[]} /> },
  property: { view: "reference", render: (data: unknown[]) => <PropertiesTable properties={data as Property[]} /> },
  scenario: { view: "matrix", render: (data: unknown[]) => <ScenarioMatrix scenarios={data as Scenario[]} /> },
};
const VIEWS = {
  diagram: (data: Scenario) => <ScenarioDiagram item={data} />,
  assessment: (data: Scenario) => <ScenarioAssessment scenario={data} />,
  card: (data: Scenario) => <ScenarioCard scenario={data} />,
};
type View = keyof typeof VIEWS;

type ModelRecord = { id: string; meta: { data: unknown; order: number } };

// Read on each call, so a rerun of threat-model.mts shows on the next request in
// `next dev`. The file is small, and a page cites a few records at most.
function modelRecords(id: string): ModelRecord[] {
  if (!fs.existsSync(MODEL)) throw new Error(`<Snippet id="${id}">: ${MODEL} is missing; run node lib/snippets/threat-model.mts --out .nym-codex/build/model.jsonl`);
  return fs.readFileSync(MODEL, "utf-8").split("\n").filter(Boolean).map((line) => JSON.parse(line));
}

function modelRecord(id: string): ModelRecord {
  const record = modelRecords(id).find((r) => r.id === id);
  if (!record) throw new Error(`<Snippet id="${id}">: no threat-model record with this id`);
  return record;
}

export function Snippet({ id, view }: { id: string; view?: View | "reference" | "matrix" }) {
  if (!id.startsWith(REPO)) throw new Error(`<Snippet id="${id}">: not a snippet of this repository`);
  if (id.includes("#")) throw new Error(`<Snippet id="${id}">: code citations are not rendered on the docs site yet`);
  const kind = id.match(/\/threat-model\/(\w+)$/)?.[1];
  if (kind !== undefined && Object.hasOwn(COLLECTIONS, kind)) {
    const collection = COLLECTIONS[kind as keyof typeof COLLECTIONS];
    if (view !== collection.view) throw new Error(`<Snippet id="${id}">: a kind citation needs view="${collection.view}"`);
    const prefix = `${id}/`;
    const data = modelRecords(id).filter((r) => r.id.startsWith(prefix)).sort((a, b) => a.meta.order - b.meta.order).map((r) => r.meta.data);
    if (!data.length) throw new Error(`<Snippet id="${id}">: no record of this kind`);
    return collection.render(data);
  }
  const model = id.includes("/threat-model/");
  if (model && view === undefined) throw new Error(`<Snippet id="${id}">: a threat-model citation needs view="${Object.keys(VIEWS).join('" | "')}"`);
  if (view !== undefined) {
    if (!model) throw new Error(`<Snippet id="${id}" view="${view}">: only a threat-model scenario or variant has a view`);
    if (!Object.hasOwn(VIEWS, view)) throw new Error(`<Snippet id="${id}" view="${view}">: the view is not one of ${Object.keys(VIEWS).join(", ")}`);
    const variant = id.includes("/threat-model/variant/");
    if (!variant && !/\/threat-model\/(example\/[^/]+\/)?scenario\//.test(id)) throw new Error(`<Snippet id="${id}" view="${view}">: only a scenario or a scenario variant has views`);
    if (variant && !VARIANT_VIEWS.includes(view)) throw new Error(`<Snippet id="${id}" view="${view}">: a scenario variant has the view ${VARIANT_VIEWS.join(", ")} only`);
    return VIEWS[view as View](modelRecord(id).meta.data as Scenario);
  }
  const file = path.join(SNIPPETS, `${id.slice(REPO.length)}.mmd`);
  if (!fs.existsSync(file)) throw new Error(`<Snippet id="${id}">: no diagram at ${file}`);
  return <Mermaid chart={fs.readFileSync(file, "utf-8")} />;
}
