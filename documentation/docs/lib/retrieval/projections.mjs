// Project component-rendered content into text the retrieval build can see.
//
// The threat-model pages are deliberately thin: the prose introduces a concept
// and a component renders the substance from typed data. `<ActorsReference />`
// is the entire definition of L2, L3L and L3G; the page around it is two
// sentences of preamble.
//
// The chunker reads MDX source, so without a projection it sees the tag and
// nothing else, and the canonical definitions of the actors, vectors and
// properties, which every other page refers to by name, are absent from the
// index entirely.
//
// Rather than render React in the build, each component that carries content is
// paired with a projection of the same data into markdown. The data is read from
// the same typed modules the components use, so the two cannot drift: adding an
// actor updates the page and the index together.
//
// Components that are purely interactive (the animated diagrams) project their
// caption and nothing else. There is no text in an animation to lose.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadTsModule } from './ts-data.mjs';
import { modelCitation } from '../snippets/repo.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const MODEL = path.join(HERE, '..', 'privacy-model');

// Fields in the spine are string-or-array depending on the entry (`requires` is
// a bare string on some scenarios and a list on others), so normalise rather
// than assume. Getting this wrong drops content silently, which is the failure
// this whole module exists to fix.
const list = (x) => (x == null ? [] : Array.isArray(x) ? x : [x]);
const bullets = (xs) => list(xs).map((x) => `- ${x}`).join('\n');

/** The text of one actor, below its heading. Shared with the threat-model records. */
export function actorText(a) {
  const lines = [a.vantage, ''];
  if (a.observes?.length) lines.push('Observes:', '', bullets(a.observes), '');
  if (a.cannotObserveMixnet?.length) {
    lines.push('Cannot observe (direct connection):', '', bullets(a.cannotObserve), '');
    lines.push('Cannot observe (mixnet):', '', bullets(a.cannotObserveMixnet), '');
  } else if (a.cannotObserve?.length) {
    lines.push('Cannot observe:', '', bullets(a.cannotObserve), '');
  }
  if (a.cost) lines.push(`Cost to become: ${a.cost}`, '');
  return lines.join('\n').trim();
}

/** The text of one vector, below its heading. */
export function vectorText(v) {
  const lines = [v.consistsOf, ''];
  if (v.observableFrom?.length) lines.push(`Observable from: ${v.observableFrom.join(', ')}.`, '');
  if (v.countermeasures?.length) {
    lines.push('Countermeasures:', '');
    lines.push(
      v.countermeasures
        .map((c) => {
          const against = c.against?.length ? ` (against ${c.against.join(', ')})` : '';
          const layer = c.layer ? ` [${c.layer} layer]` : '';
          return `- ${c.text}${against}${layer}`;
        })
        .join('\n'),
      '',
    );
  }
  return lines.join('\n').trim();
}

function projectActors(ACTORS) {
  const body = ACTORS.map((a) => {
    // `{#actor-L2}` matches the id the ThreatModelReference component renders,
    // so a citation lands on the card a reader can actually see.
    const lines = [`### ${a.id}: ${a.name}${a.primary ? ' (the primary adversary)' : ''} {#actor-${a.id}}`, '', actorText(a), ''];
    return lines.join('\n');
  }).join('\n');
  return `## The actors\n\n${body}`;
}

function projectVectors(VECTORS) {
  const body = VECTORS.map((v) => `### ${v.id}: ${v.name} {#vector-${v.id}}\n\n${vectorText(v)}\n`).join('\n');
  return `## The vectors\n\n${body}`;
}

function projectProperties(PROPERTIES) {
  const body = PROPERTIES.map((p) => `### ${p.id}: ${p.name} {#prop-${p.id}}\n\n${p.definition}\n`).join('\n');
  return `## The properties\n\n${body}`;
}

function projectLayers(LAYERS) {
  const body = (LAYERS ?? [])
    .map((l) => {
      const lines = [`### ${l.title}`, '', l.summary, ''];
      if (l.provides?.length) lines.push('Provides:', '', bullets(l.provides), '');
      return lines.join('\n');
    })
    .join('\n');
  return `## The layers\n\n${body}`;
}

/** Verdict keys are `p1L2`, `p2L3G` and so on: property, then actor. */
function matrixRows(matrix) {
  return Object.entries(matrix ?? {})
    .map(([key, cell]) => {
      const m = key.match(/^(p\d)(L.+)$/i);
      const label = m ? `${m[1].toUpperCase()} against ${m[2]}` : key;
      return `- ${label}: ${cell.verdict}${cell.text ? ` (${cell.text})` : ''}`;
    })
    .join('\n');
}

/** The routes of a scenario as ordered hops, which the diagram draws. Shared with the threat-model records. */
export function routesText(s) {
  const routes = list(s.paths).map((p) => `- ${p.id} (${p.mode}): ${list(p.stages).join(' -> ')}`);
  return routes.length ? `Routes, hop by hop:\n\n${routes.join('\n')}` : '';
}

/**
 * `heading` controls whether the projection opens its own `## <title>` section.
 *
 * It must not, when the trigger is a component that sits mid-section. A synthetic
 * H2 becomes a section boundary for the chunker, so every paragraph between it and
 * the next real heading is re-filed under it, which can put surrounding prose under
 * a heading that contradicts it. That is worse than not indexing the diagram at all.
 */
function projectScenario(s, { heading = true } = {}) {
  if (!s) return '';
  const lines = heading ? [`## ${s.title}`, '', s.summary, ''] : [s.summary, ''];
  if (s.topology?.caption) lines.push(s.topology.caption, '');
  const routes = routesText(s);
  if (routes) lines.push(routes, '');
  if (s.matrix) lines.push('Verdicts by property and actor:', '', matrixRows(s.matrix), '');
  for (const a of list(s.actorAssessment)) {
    lines.push(`Against ${a.actor}:`, '');
    if (list(a.sees).length) lines.push(`- Sees: ${list(a.sees).join('; ')}`);
    if (list(a.cantSee).length) lines.push(`- Cannot see: ${list(a.cantSee).join('; ')}`);
    for (const [k, v] of Object.entries(a)) {
      if (/^p\d$/.test(k)) lines.push(`- ${k.toUpperCase()}: ${v}`);
    }
    if (list(a.residual).length) lines.push(`- Residual risk: ${list(a.residual).join(' ')}`);
    lines.push('');
  }
  if (list(s.requires).length) lines.push('Requires:', '', bullets(s.requires), '');
  if (list(s.pros).length) lines.push('Strengths:', '', bullets(s.pros), '');
  if (list(s.cons).length) lines.push('Weaknesses:', '', bullets(s.cons), '');
  if (list(s.fit).length) lines.push('Fit:', '', bullets(s.fit), '');
  return lines.join('\n');
}

// The component each view of a scenario citation renders (components/snippet.tsx).
export const VIEW_COMPONENTS = { diagram: 'NetworkDiagram', assessment: 'GenericScenarioView', card: 'MetadataPanel' };
// The component each kind citation renders (`<Snippet id=".../threat-model/actor" view="reference" />`).
export const COLLECTION_COMPONENTS = { actor: 'ActorsReference', vector: 'VectorsReference', property: 'PropertiesReference', scenario: 'ConfigMatrix' };

/**
 * The scenario a citation tag names (`<Snippet id=".../scenario/mixnet" view="diagram" />`),
 * as `{ component, ref }`, or null for a tag that cites no threat-model record.
 * A scenario variant (`.../variant/<id>`) projects as the scenario it resolves to,
 * under the key of its base, so a page that shows a base and its variants carries
 * the base text once. A wrong view is an error, which the record check
 * (citationText) reports first.
 */
function citedScenario(attrs) {
  const id = attrs.match(/\bid="([^"]+)"/)?.[1] ?? '';
  const view = attrs.match(/\bview="([^"]+)"/)?.[1];
  const model = modelCitation(id);
  if (!model) return null;
  if (!model.views.includes(view)) throw new Error(`<Snippet id="${id}">: view "${view}" is not one of ${model.views.join(', ') || 'none'}`);
  if (model.kind === 'collection') return { component: COLLECTION_COMPONENTS[model.type], ref: null };
  const component = VIEW_COMPONENTS[view];
  return { component, ref: { source: model.source, id: model.id } };
}

/**
 * Build the expander used by stripMdx.
 *
 * Returns `(tagName, attrs, ctx) => string | null`. Null means "no projection
 * for this component", and the tag is dropped, which is correct for anything
 * whose content is purely visual.
 *
 * A scenario comes from a citation (`<Snippet id view>`, see citedScenario), or
 * from the page's own `export const x = requireGenericScenario('id')` lines
 * (`ctx.scenarios`, `ctx.firstScenario`, `ctx.scenarioId`).
 */
export async function loadProjections() {
  const tm = await loadTsModule(path.join(MODEL, 'threat-model.ts'));
  const gen = await loadTsModule(path.join(MODEL, 'examples', 'generic.ts'));

  // source -> id -> scenario, for the generic scenarios and the three worked examples.
  const scenarioBy = { generic: new Map((gen.GENERIC_SCENARIOS ?? []).map((s) => [s.id, s])) };
  const exampleExtras = {};
  // A variant resolves to a full scenario, keyed by its base for the dedupe.
  const variants = await loadTsModule(path.join(MODEL, 'examples', 'variants.ts'));
  scenarioBy.variant = new Map(variants.SCENARIO_VARIANTS.map((v) => [v.id, variants.resolveScenarioVariant(v)]));
  const variantBase = new Map(variants.SCENARIO_VARIANTS.map((v) => [v.id, v.base]));
  for (const name of ['browsing', 'messaging', 'wallet']) {
    const ex = await loadTsModule(path.join(MODEL, 'examples', `${name}.ts`));
    const upper = name.toUpperCase();
    scenarioBy[name] = new Map(list(ex[`${upper}_SCENARIOS`]).map((s) => [s.id, s]));
    exampleExtras[name] = projectExampleExtras(ex[`${upper}_L1`], ex[`${upper}_INVARIANTS`]);
  }

  const statics = {
    ActorsReference: () => projectActors(tm.ACTORS),
    VectorsReference: () => projectVectors(tm.VECTORS),
    PropertiesReference: () => projectProperties(tm.PROPERTIES),
    LayersReference: () => projectLayers(tm.LAYERS),
    // The comparison matrix sits mid-section, so it projects with no heading of its own.
    ConfigMatrix: () => projectMatrix(gen.GENERIC_SCENARIOS),
  };

  return (tag, attrs, ctx = {}) => {
    // A citation projects as the view component it renders, for the scenario its id names.
    const cited = tag === 'Snippet' ? citedScenario(attrs) : null;
    if (tag === 'Snippet' && !cited) return null;
    const tagName = cited ? cited.component : tag;
    if (statics[tagName]) return statics[tagName]();
    if (
      tagName === 'GenericScenarioView' ||
      tagName === 'MetadataPanel' ||
      tagName === 'PropertyBadges' ||
      tagName === 'NetworkDiagram'
    ) {
      // These render facets of one scenario. Projecting the whole scenario
      // once per page and letting the first one win avoids repeating it.
      // NetworkDiagram counts: its topology caption is prose that only exists
      // in the data, so a page whose only scenario component is the diagram
      // would otherwise contribute nothing about the route it draws. It
      // projects without a heading, because unlike the other three it appears
      // mid-section.
      //
      // The scenario is the one the tag names through its props
      // (`item={mixnet}`), else the page's first, so a page that shows two
      // scenarios projects both, each once.
      // A tag with no scenario prop takes the page's first scenario; a prop
      // that names no plain scenario const (a spread, a typo) projects nothing.
      const named = attrs.match(/\b(?:item|scenario)=\{\s*(\w+)\s*\}/)?.[1];
      const ref = cited ? cited.ref : named !== undefined
        ? ctx.scenarios?.get(named) ?? null
        : ctx.firstScenario ?? (ctx.scenarioId ? { source: 'generic', id: ctx.scenarioId } : null);
      const s = ref && scenarioBy[ref.source]?.get(ref.id);
      if (cited && !s) throw new Error(`<Snippet ${attrs.trim()}>: no scenario ${ref.source}/${ref.id} in the model`);
      if (!s) return null;
      ctx.projected ??= new Set();
      const key = ref.source === 'variant' ? `generic/${variantBase.get(ref.id)}` : `${ref.source}/${ref.id}`;
      if (ctx.projected.has(key)) return null;
      ctx.projected.add(key);
      // A scenario titled like its page is the page itself: no heading of its own,
      // or the chunker files it as a section whose anchor the page does not render.
      let text = projectScenario(s, { heading: tagName !== 'NetworkDiagram' && s.title !== ctx.pageTitle });
      // A worked example's own adversary and invariants, once on its page.
      if (exampleExtras[ref.source] && !ctx.projected.has(`${ref.source}/extras`)) {
        ctx.projected.add(`${ref.source}/extras`);
        text = `${text}\n${exampleExtras[ref.source]}`;
      }
      return text;
    }
    return null;
  };
}

/**
 * Constants pages interpolate, such as `{RUST_MSRV}`.
 *
 * MDX evaluates these at render time; the chunker cannot, so without substitution
 * the index carries the literal `{RUST_MSRV}` instead of a version. That is worse
 * than carrying nothing, because the page then states a requirement that cannot
 * be read, and the reader has no way to tell a placeholder from a value. The real
 * values are one import away, so substitute them.
 */
export async function loadDocValues() {
  const mod = await loadTsModule(path.join(HERE, '..', '..', 'components', 'versions.ts'));
  const values = {};
  for (const [k, v] of Object.entries(mod)) {
    if (typeof v === 'string' || typeof v === 'number') values[k] = String(v);
  }
  // The network counts that `<Stat name="nodes" />` shows (components/stat.tsx), as `stat.<name>`.
  // The file is in git, and components/stat.tsx imports it too, so a rename fails both.
  const statsFile = path.join(HERE, '..', '..', 'components', 'outputs', 'api-scraping-outputs', 'nodes-count.json');
  for (const [k, v] of Object.entries(JSON.parse(fs.readFileSync(statsFile, 'utf-8')))) {
    if (typeof v === 'string' || typeof v === 'number') values[`stat.${k}`] = String(v);
  }
  return values;
}

/** A worked example's L1 actor and invariants. */
function projectExampleExtras(l1, invariants) {
  const lines = [];
  if (l1) lines.push(`Adversary ${l1.id}, ${l1.name}:`, '', actorText(l1), '');
  if (list(invariants).length) {
    lines.push('Invariants:', '', list(invariants).map((i) => `- ${i.id}, ${i.name}: ${i.statement}`).join('\n'), '');
  }
  return lines.join('\n');
}

/** The comparison matrix: each generic scenario with its verdicts by property and actor. */
function projectMatrix(scenarios) {
  const rows = list(scenarios).map((s) => `${s.title}:\n\n${matrixRows(s.matrix)}`);
  return `Verdicts by configuration, property, and actor:\n\n${rows.join('\n\n')}\n`;
}

export const __test = { projectActors, projectVectors, projectProperties, projectScenario, matrixRows };
