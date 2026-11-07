// Threat-model adapter (D4b, tasks 6b.2 and 6b.4): one `model` record for each
// entity of the typed data in lib/privacy-model, and the integrity check.
//
//   node lib/snippets/threat-model.mts                 print the records and the integrity report
//   node lib/snippets/threat-model.mts --out <file>    also write the records as JSON lines; the docs
//                                                      build and dev server read them (components/snippet.tsx)
//
// Ids: nym/threat-model/<type>/<id>, and nym/threat-model/example/<name>/<type>/<id>
// for the worked examples. A scenario variant (examples/variants.ts, another drawing
// of a generic scenario) is nym/threat-model/variant/<id>. A claim is nym/threat-model/claim/<scenario>/<property>/<actor>. The text in `body` is the same projection the docs
// index uses (projections.mjs), and `meta.data` holds the evaluated entity.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';
import { loadTsModule } from '../retrieval/ts-data.mjs';
import { actorText, vectorText, __test } from '../retrieval/projections.mjs';
import type { ActorAssessment, Invariant, MatrixCell, Property, PropertyId, Scenario, ScenarioVariant, ThreatActor, Vector } from '../privacy-model/types.ts';
import type { Snippet, SnippetRef } from '../retrieval/types.ts';

// The modules that loadTsModule evaluates. It returns them untyped, so these
// shapes are a statement of their exports; the entity types are the source's own.
interface ThreatModelModule { ACTORS: ThreatActor[]; VECTORS: Vector[]; PROPERTIES: Property[] }
interface GenericModule { GENERIC_SCENARIOS: Scenario[] }
interface VariantsModule { SCENARIO_VARIANTS: ScenarioVariant[]; resolveScenarioVariant: (v: ScenarioVariant) => Scenario }
/** A worked example exports `<NAME>_L1`, `<NAME>_INVARIANTS`, and `<NAME>_SCENARIOS`. */
type ExampleModule = Record<string, unknown>;

const HERE = path.dirname(fileURLToPath(import.meta.url));
const MODEL = path.resolve(HERE, '../privacy-model');
const SOURCE = 'documentation/docs/lib/privacy-model';
const PROVENANCE = 'The note "Network privacy" by Claudia Diaz, sections 2, 3, and 5';
const EXAMPLES = ['browsing', 'messaging', 'wallet'];
const SPINE = ['L2', 'L3L', 'L3G'];

// Scenarios that leave out an assessment on purpose, with the reason (P15:
// the threat-model owner fills this list).
export const ASSESSMENT_EXCEPTIONS = [
  { scenario: 'end-to-end', actor: 'L2', reason: 'Known gap: the end-to-end scenario has no L2 assessment. Noted by this change, not fixed (design D4b).' },
];

const sha256 = (text: string): string => crypto.createHash('sha256').update(text).digest('hex');
const list = <T,>(x: T | T[] | null | undefined): T[] => (x == null ? [] : Array.isArray(x) ? x : [x]);

interface RecordInput {
  id: string;
  title: string;
  body: string;
  file: string;
  data: unknown;
  refs?: SnippetRef[];
}

function record(repo: string, { id, title, body, file, data, refs = [] }: RecordInput): Snippet {
  const seen = new Set<string>();
  const sorted = refs
    .filter((r) => { const k = `${r.id}\0${r.rel}`; if (seen.has(k)) return false; seen.add(k); return true; })
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : a.rel < b.rel ? -1 : a.rel > b.rel ? 1 : 0));
  return {
    id: `${repo}/threat-model/${id}`,
    kind: 'model',
    authority: 'normative',
    repo,
    path: `${SOURCE}/${file}`,
    title,
    body,
    hash: sha256(body),
    refs: sorted,
    meta: { data: JSON.parse(JSON.stringify(data)), source: `${SOURCE}/${file}`, provenance: PROVENANCE },
  };
}

/** Matrix key `p1L2` -> { property: 'P1', actor: 'L2' }. */
function matrixKey(key: string): { property: string; actor: string } | null {
  const [, property, actor] = key.match(/^(p\d)(L.+)$/i) ?? [];
  return property !== undefined && actor !== undefined ? { property: property.toUpperCase(), actor } : null;
}

/** The field of a per-actor assessment that states the verdict for each property. A new PropertyId fails tsc here. */
const VERDICT_FIELD: Record<PropertyId, 'p1' | 'p2'> = { P1: 'p1', P2: 'p2' };
const hasVerdictField = (property: string): property is PropertyId => Object.hasOwn(VERDICT_FIELD, property);

// The cells of a matrix row. `ScenarioMatrix` is an interface, so Object.entries cannot type its values.
const cells = (s: Scenario): [string, MatrixCell][] => Object.entries(s.matrix ?? {}) as [string, MatrixCell][];

export interface ThreatModelBuild {
  records: Snippet[];
  errors: string[];
  warnings: string[];
}

interface ScenarioOptions {
  prefix: string;
  file: string;
  /** Actor id -> record id: the spine actors, and the L1 of a worked example. */
  actors: Map<string, string>;
  spine: string[];
  specialises?: string;
}

export async function buildThreatModel({ repo = 'nym' }: { repo?: string } = {}): Promise<ThreatModelBuild> {
  const tm: ThreatModelModule = await loadTsModule(path.join(MODEL, 'threat-model.ts'));
  const gen: GenericModule = await loadTsModule(path.join(MODEL, 'examples', 'generic.ts'));
  const variants: VariantsModule = await loadTsModule(path.join(MODEL, 'examples', 'variants.ts'));
  const examples: Record<string, ExampleModule> = {};
  for (const name of EXAMPLES) examples[name] = await loadTsModule(path.join(MODEL, 'examples', `${name}.ts`));

  const records: Snippet[] = [];
  const claims: Snippet[] = [];
  const errors: string[] = [];
  const warnings: string[] = [];
  const id = (type: string, key: string): string => `${repo}/threat-model/${type}/${key}`;
  const actorIds = new Set<string>(tm.ACTORS.map((a) => a.id));
  const propertyIds = new Set<string>(tm.PROPERTIES.map((p) => p.id));

  for (const a of tm.ACTORS) {
    const observes = tm.VECTORS.filter((v) => list(v.observableFrom).includes(a.id)).map((v): SnippetRef => ({ id: id('vector', v.id), rel: 'observes' }));
    records.push(record(repo, { id: `actor/${a.id}`, title: `${a.id}: ${a.name}`, body: actorText(a), file: 'threat-model.ts', data: a, refs: observes }));
  }
  for (const v of tm.VECTORS) {
    for (const actor of [...list(v.observableFrom), ...list(v.countermeasures).flatMap((c) => list(c.against))]) {
      if (!actorIds.has(actor)) errors.push(`vector ${v.id}: actor ${actor} does not exist`);
    }
    records.push(record(repo, { id: `vector/${v.id}`, title: `${v.id}: ${v.name}`, body: vectorText(v), file: 'threat-model.ts', data: v }));
  }
  for (const p of tm.PROPERTIES) {
    records.push(record(repo, { id: `property/${p.id}`, title: `${p.id}: ${p.name}`, body: p.definition, file: 'threat-model.ts', data: p }));
  }

  // A scenario: its text, its edges to the actors and properties it is assessed
  // against, and the integrity rules of D4b.
  const scenario = (s: Scenario, { prefix, file, actors, spine, specialises }: ScenarioOptions): Snippet => {
    const where = `${prefix}scenario ${s.id}`;
    const refs: SnippetRef[] = [];
    const assessed = new Map<string, ActorAssessment>(list(s.actorAssessment).map((a) => [a.actor, a]));
    for (const a of assessed.keys()) {
      const actor = actors.get(a);
      if (actor === undefined) errors.push(`${where}: assessment of actor ${a}, which does not exist`);
      else refs.push({ id: actor, rel: 'assessed-against' });
    }
    let matrixOnly = 0;
    for (const [key, cell] of cells(s)) {
      const k = matrixKey(key);
      if (!k || !propertyIds.has(k.property) || !actors.has(k.actor)) {
        errors.push(`${where}: matrix key ${key} names no property and actor`);
        continue;
      }
      refs.push({ id: id('property', k.property), rel: 'assessed-against' });
      if (!hasVerdictField(k.property)) {
        errors.push(`${where}: property ${k.property} has no field in the per-actor assessment`);
        continue;
      }
      const stated = assessed.get(k.actor)?.[VERDICT_FIELD[k.property]];
      if (stated === undefined) matrixOnly++;
      else if (stated !== cell.verdict) errors.push(`${where}: ${k.property} against ${k.actor} is "${cell.verdict}" in the matrix and "${stated}" in the assessment`);
    }
    if (matrixOnly) warnings.push(`${where}: ${matrixOnly} matrix verdicts are not in the per-actor assessment`);
    for (const a of spine) {
      if (assessed.has(a)) continue;
      const exception = ASSESSMENT_EXCEPTIONS.find((e) => e.scenario === s.id && e.actor === a && !prefix);
      if (exception) warnings.push(`${where}: no ${a} assessment, on the exception list: ${exception.reason}`);
      else errors.push(`${where}: no assessment for spine actor ${a}, and not on the exception list`);
    }
    if (specialises) refs.push({ id: specialises, rel: 'specialises' });
    // Each cell of the comparison matrix is one claim (D15, task 6b.9): a
    // configuration, a property, an actor, a verdict, and its condition.
    const scenarioId = `${prefix.replace(/ /g, '/')}scenario/${s.id}`;
    for (const [key, cell] of cells(s)) {
      const k = matrixKey(key);
      const actor = k ? actors.get(k.actor) : undefined;
      if (!k || actor === undefined) continue;
      const claim = `${prefix.replace(/ /g, '/')}claim/${s.id}/${k.property}/${k.actor}`;
      const body = `${s.title}: ${k.property} against ${k.actor} is "${cell.verdict}"${cell.text ? `, ${cell.text}` : ''}.`;
      claims.push(record(repo, {
        id: claim,
        title: `${s.shortTitle ?? s.title}: ${k.property} against ${k.actor}`,
        body,
        file,
        data: { scenario: s.id, property: k.property, actor: k.actor, ...cell },
        refs: [
          { id: `${repo}/threat-model/${scenarioId}`, rel: 'relates-to' },
          { id: actor, rel: 'assessed-against' },
          { id: id('property', k.property), rel: 'assessed-against' },
        ],
      }));
    }
    // The same projection the docs index uses, routes included.
    const body = __test.projectScenario(s, { heading: false }).trim();
    return record(repo, { id: `${prefix.replace(/ /g, '/')}scenario/${s.id}`, title: s.title, body, file, data: s, refs });
  };

  const spineActors = new Map<string, string>(tm.ACTORS.map((a) => [a.id, id('actor', a.id)]));
  const generic = new Map<string, string>();
  for (const s of gen.GENERIC_SCENARIOS) {
    generic.set(s.id, id('scenario', s.id));
    records.push(scenario(s, { prefix: '', file: 'examples/generic.ts', actors: spineActors, spine: SPINE }));
  }

  // A variant holds the routes and verdicts of its base, so it states no claim of
  // its own: its record is the drawing, with a `specialises` edge to the base.
  for (const v of variants.SCENARIO_VARIANTS) {
    const base = generic.get(v.base);
    if (base === undefined) { errors.push(`scenario variant ${v.id}: base ${v.base} is not a generic scenario`); continue; }
    const s = variants.resolveScenarioVariant(v);
    const body = `A drawing of the scenario "${s.title}" for one page. It has the routes and verdicts of that scenario.${s.topology?.caption ? ` ${s.topology.caption}` : ''}`;
    records.push(record(repo, { id: `variant/${v.id}`, title: `${s.title}: ${v.id}`, body, file: 'examples/variants.ts', data: s, refs: [{ id: base, rel: 'specialises' }] }));
  }

  for (const name of EXAMPLES) {
    const ex = examples[name];
    const upper = name.toUpperCase();
    const l1 = ex?.[`${upper}_L1`] as ThreatActor | undefined;
    if (!ex || !l1) { errors.push(`worked example ${name}: examples/${name}.ts exports no ${upper}_L1`); continue; }
    const base = `example/${name}`;
    const actors = new Map([...spineActors, [l1.id, `${repo}/threat-model/${base}/actor/${l1.id}`]]);
    records.push(record(repo, { id: `${base}/actor/${l1.id}`, title: `${l1.id}: ${l1.name}`, body: actorText(l1), file: `examples/${name}.ts`, data: l1 }));
    for (const inv of list(ex[`${upper}_INVARIANTS`] as Invariant[] | undefined)) {
      records.push(record(repo, { id: `${base}/invariant/${inv.id}`, title: `Invariant ${inv.id}: ${inv.name}`, body: inv.statement, file: `examples/${name}.ts`, data: inv }));
    }
    for (const s of list(ex[`${upper}_SCENARIOS`] as Scenario[] | undefined)) {
      // A worked-example scenario specialises the generic one with the same id; the data names no other link.
      records.push(scenario(s, { prefix: `${base} `, file: `examples/${name}.ts`, actors, spine: [], specialises: generic.get(s.id) }));
    }
  }

  // A claim is a statement: no RFC 2119 level, and approved until a person enforces it.
  for (const c of claims) c.meta = { ...c.meta, status: 'approved' };
  records.push(...claims);
  const ids = new Set(records.map((r) => r.id));
  for (const r of records) for (const ref of r.refs) if (!ids.has(ref.id)) errors.push(`${r.id}: edge to ${ref.id}, which has no record`);
  // The position in the source data, before the sort by id: a table of a whole kind
  // (components/snippet.tsx) shows the records in the order the model gives them.
  records.forEach((r, i) => { r.meta = { ...r.meta, order: i }; });
  records.sort((a, b) => (a.id < b.id ? -1 : 1));
  return { records, errors, warnings };
}

async function main(argv: string[]): Promise<void> {
  const { out } = parseArgs({ args: argv, options: { out: { type: 'string' } } }).values;
  const { records, errors, warnings } = await buildThreatModel();
  const byType: Record<string, number> = {};
  for (const r of records) {
    // The entity type follows `threat-model/` or `example/<name>/`.
    const type = r.id.replace(/^[^/]+\/threat-model\/(example\/[^/]+\/)?/, '').split('/')[0] ?? '';
    byType[type] = (byType[type] ?? 0) + 1;
  }
  for (const e of errors) console.log(`error  ${e}`);
  for (const w of warnings) console.log(`warn   ${w}`);
  console.log(`${records.length} model records: ${Object.entries(byType).map(([t, n]) => `${n} ${t}`).join(', ')}`);
  console.log(`${errors.length} integrity errors, ${warnings.length} warnings`);
  if (out && !errors.length) {
    fs.mkdirSync(path.dirname(out), { recursive: true });
    fs.writeFileSync(out, records.map((r) => JSON.stringify(r)).join('\n') + '\n');
    console.log(`wrote ${out}`);
  }
  process.exitCode = errors.length ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
