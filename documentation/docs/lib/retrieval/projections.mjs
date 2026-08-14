// Project component-rendered content into text the retrieval build can see.
//
// The threat-model pages are deliberately thin: the prose introduces a concept
// and a component renders the substance from typed data. `<ActorsReference />`
// is the entire definition of L2, L3L and L3G; the page around it is two
// sentences of preamble.
//
// The chunker reads MDX source, so it saw the tag and nothing else. The canonical
// definitions of the actors, vectors and properties, which every other page
// refers to by name, were absent from the index entirely. Retrieval answered
// questions about them from commentary elsewhere, or not at all.
//
// Rather than render React in the build, each component that carries content is
// paired with a projection of the same data into markdown. The data is read from
// the same typed modules the components use, so the two cannot drift: adding an
// actor updates the page and the index together.
//
// Components that are purely interactive (the animated diagrams) project their
// caption and nothing else. There is no text in an animation to lose.

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadTsModule } from './ts-data.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const MODEL = path.join(HERE, '..', 'privacy-model');

// Fields in the spine are string-or-array depending on the entry (`requires` is
// a bare string on some scenarios and a list on others), so normalise rather
// than assume. Getting this wrong drops content silently, which is the failure
// this whole module exists to fix.
const list = (x) => (x == null ? [] : Array.isArray(x) ? x : [x]);
const bullets = (xs) => list(xs).map((x) => `- ${x}`).join('\n');

function projectActors(ACTORS) {
  const body = ACTORS.map((a) => {
    // `{#actor-L2}` matches the id the ThreatModelReference component renders,
    // so a citation lands on the card a reader can actually see.
    const lines = [`### ${a.id}: ${a.name}${a.primary ? ' (the primary adversary)' : ''} {#actor-${a.id}}`, '', a.vantage, ''];
    if (a.observes?.length) lines.push('Observes:', '', bullets(a.observes), '');
    if (a.cannotObserve?.length) lines.push('Cannot observe:', '', bullets(a.cannotObserve), '');
    if (a.cost) lines.push(`Cost to become: ${a.cost}`, '');
    return lines.join('\n');
  }).join('\n');
  return `## The actors\n\n${body}`;
}

function projectVectors(VECTORS) {
  const body = VECTORS.map((v) => {
    const lines = [`### ${v.id}: ${v.name} {#vector-${v.id}}`, '', v.consistsOf, ''];
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
    return lines.join('\n');
  }).join('\n');
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

function projectScenario(s) {
  if (!s) return '';
  const lines = [`## ${s.title}`, '', s.summary, ''];
  if (s.topology?.caption) lines.push(s.topology.caption, '');
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

/**
 * Build the expander used by stripMdx.
 *
 * Returns `(tagName, attrs, ctx) => string | null`. Null means "no projection
 * for this component", and the tag is dropped as before, which is correct for
 * anything whose content is purely visual.
 *
 * `ctx.scenarioId` comes from the page's own
 * `export const scenario = requireGenericScenario('id')` line.
 */
export async function loadProjections() {
  const tm = await loadTsModule(path.join(MODEL, 'threat-model.ts'));
  const gen = await loadTsModule(path.join(MODEL, 'examples', 'generic.ts'));

  const scenarioById = new Map((gen.GENERIC_SCENARIOS ?? []).map((s) => [s.id, s]));

  const statics = {
    ActorsReference: () => projectActors(tm.ACTORS),
    VectorsReference: () => projectVectors(tm.VECTORS),
    PropertiesReference: () => projectProperties(tm.PROPERTIES),
    LayersReference: () => projectLayers(tm.LAYERS),
  };

  return (tagName, _attrs, ctx = {}) => {
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
      // contributed nothing about the route it draws.
      if (!ctx.scenarioId || ctx.scenarioProjected) return null;
      ctx.scenarioProjected = true;
      return projectScenario(scenarioById.get(ctx.scenarioId));
    }
    return null;
  };
}

/**
 * Constants pages interpolate, such as `{RUST_MSRV}`.
 *
 * MDX evaluates these at render time; the chunker cannot, so the index carried
 * the literal `{RUST_MSRV}` instead of a version. That is worse than carrying
 * nothing, because the page then states a requirement that cannot be read, and
 * the reader has no way to tell a placeholder from a value. The real values are
 * one import away, so substitute them.
 */
export async function loadDocValues() {
  const mod = await loadTsModule(path.join(HERE, '..', '..', 'components', 'versions.ts'));
  const values = {};
  for (const [k, v] of Object.entries(mod)) {
    if (typeof v === 'string' || typeof v === 'number') values[k] = String(v);
  }
  return values;
}

export const __test = { projectActors, projectVectors, projectProperties, projectScenario, matrixRows };
