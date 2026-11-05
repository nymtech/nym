// Tracey markers as codex edges (spike, branch max/tracey-spike).
//
//   node lib/snippets/tracey.mts [--root <repo root>] [--compare <edges file>]
//
// Tracey (github.com/bearcove/tracey) marks a requirement in a spec with `r[id]` on its own line,
// and code that relates to it with a comment `// r[VERB id]`. This tool reads the same markers and
// turns them into codex records: `impl` gives a `realises` edge from the code item to the requirement,
// `verify` names the test as the requirement's witness, `depends` and `related` give `depends-on` and
// `relates-to`. A marker on the line above an item belongs to that item, as in Tracey.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { slug } from 'github-slugger';
import { items, matchSymbol } from '../retrieval/items.mjs';
import { openRepo } from './repo.mts';

const VERBS = ['impl', 'verify', 'depends', 'related'] as const;
type Verb = (typeof VERBS)[number];

const RELS: Record<Exclude<Verb, 'verify'>, string> = { impl: 'realises', depends: 'depends-on', related: 'relates-to' };

/** A requirement defined in a spec: its Tracey id and the codex id of the requirement block it sits in. */
export interface SpecMarker {
  traceyId: string;
  version: number | null;
  requirement: string;
  line: number;
}

/** A reference in a code comment. The default verb is `impl`, as in Tracey. */
export interface CodeMarker {
  traceyId: string;
  version: number | null;
  verb: Verb;
  line: number;
}

const ID = '([a-z0-9][a-z0-9._-]*)(?:\\+(\\d+))?';
const SPEC_MARKER = new RegExp(`^>?\\s*r\\[${ID}\\]\\s*$`);
const CODE_MARKER = new RegExp(`//\\s*r\\[(?:(${VERBS.join('|')})\\s+)?${ID}\\]`);

/** The `r[id]` markers of one OpenSpec spec file, each with the requirement block it belongs to. */
export function specMarkers(text: string, repo: string, spec: string): SpecMarker[] {
  const out: SpecMarker[] = [];
  let requirement: string | null = null;
  text.split('\n').forEach((line, i) => {
    const heading = line.match(/^### Requirement: (.+)$/);
    if (heading?.[1]) requirement = `${repo}/openspec/${spec}/${slug(heading[1].trim())}`;
    const m = line.match(SPEC_MARKER);
    if (m?.[1] && requirement) out.push({ traceyId: m[1], version: m[2] ? Number(m[2]) : null, requirement, line: i + 1 });
  });
  return out;
}

/** The `// r[VERB id]` markers of one source file. */
export function codeMarkers(text: string): CodeMarker[] {
  const out: CodeMarker[] = [];
  text.split('\n').forEach((line, i) => {
    const m = line.match(CODE_MARKER);
    if (m?.[2]) out.push({ verb: (m[1] as Verb | undefined) ?? 'impl', traceyId: m[2], version: m[3] ? Number(m[3]) : null, line: i + 1 });
  });
  return out;
}

interface Item { qualifiedName: string; aliases: string[]; startLine: number; endLine: number }

/** The item a marker belongs to: the item that starts after the comment lines below it, or else the smallest item around it. */
function itemFor(list: Item[], lines: string[], line: number): Item | null {
  let next = line + 1;
  while (lines[next - 1]?.trimStart().startsWith('//') && !lines[next - 1]?.trimStart().startsWith('///')) next += 1;
  const below = list.find((it) => it.startLine === next);
  if (below) return below;
  const around = list.filter((it) => it.startLine <= line && line <= it.endLine);
  return around.sort((a, b) => (a.endLine - a.startLine) - (b.endLine - b.startLine))[0] ?? null;
}

function walk(dir: string, keep: (file: string) => boolean, out: string[] = []): string[] {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'target' || e.name === 'node_modules' || e.name.startsWith('.')) continue;
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, keep, out);
    else if (keep(full)) out.push(full);
  }
  return out;
}

export interface TraceyResult {
  edges: { from: string; to: string; rel: string }[];
  witnesses: { requirement: string; witness: string }[];
  unresolved: string[];
}

/** Read every spec and Rust marker under the repository root. */
export { itemFor };

export async function readMarkers(root: string, repoName: string): Promise<TraceyResult> {
  const requirements = new Map<string, string>();
  const unresolved: string[] = [];
  const specRoot = path.join(root, 'openspec');
  for (const file of walk(specRoot, (f) => f.endsWith('spec.md'))) {
    const spec = path.basename(path.dirname(file));
    for (const m of specMarkers(fs.readFileSync(file, 'utf-8'), repoName, spec)) {
      if (requirements.has(m.traceyId)) unresolved.push(`${path.relative(root, file)}:${m.line}: r[${m.traceyId}] is defined twice`);
      requirements.set(m.traceyId, m.requirement);
    }
  }
  const edges: TraceyResult['edges'] = [];
  const witnesses: TraceyResult['witnesses'] = [];
  for (const file of walk(root, (f) => f.endsWith('.rs'))) {
    const text = fs.readFileSync(file, 'utf-8');
    if (!text.includes('r[')) continue;
    const markers = codeMarkers(text);
    const lines = text.split('\n');
    if (markers.length === 0) continue;
    const list: Item[] = await items(text, 'rust');
    const rel = path.relative(root, file);
    for (const m of markers) {
      const where = `${rel}:${m.line}`;
      const requirement = requirements.get(m.traceyId);
      const item = itemFor(list, lines, m.line);
      if (!requirement) { unresolved.push(`${where}: no spec defines r[${m.traceyId}]`); continue; }
      if (!item) { unresolved.push(`${where}: no item for the marker`); continue; }
      const code = `${repoName}/${rel}#${item.qualifiedName}`;
      if (m.verb === 'verify') witnesses.push({ requirement, witness: code });
      else edges.push({ from: code, to: requirement, rel: RELS[m.verb] });
    }
  }
  return { edges, witnesses, unresolved };
}

/** `path#symbol` and the start line of the item it names, so two spellings of one item compare equal. */
async function itemKey(root: string, repoName: string, id: string, cache: Map<string, Item[]>): Promise<string | null> {
  const [file = '', symbol = ''] = id.slice(repoName.length + 1).split('#');
  if (!cache.has(file)) cache.set(file, await items(fs.readFileSync(path.join(root, file), 'utf-8'), 'rust'));
  const found: Item[] = matchSymbol(cache.get(file) ?? [], symbol);
  return found.length === 1 && found[0] ? `${file}:${found[0].startLine}` : null;
}

async function main(argv: string[]): Promise<void> {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const rootArg = at('root');
  const repo = openRepo(rootArg ? path.resolve(rootArg) : undefined);
  const result = await readMarkers(repo.root, repo.repo);
  for (const u of result.unresolved) console.log(`warn   ${u}`);
  console.log(`${result.edges.length} edges and ${result.witnesses.length} witnesses from Tracey markers`);
  const comparePath = at('compare');
  if (!comparePath) { console.log(JSON.stringify(result, null, 2)); return; }
  const parsed: unknown = JSON.parse(fs.readFileSync(comparePath, 'utf-8'));
  const hand = (Array.isArray(parsed) ? parsed : (parsed as { edges: unknown[] }).edges) as { from: string; to: string; rel: string }[];
  const cache = new Map<string, Item[]>();
  const key = async (e: { from: string; to: string; rel: string }) => `${await itemKey(repo.root, repo.repo, e.from, cache)} -> ${e.to} (${e.rel})`;
  const fromMarkers = new Set(await Promise.all(result.edges.map(key)));
  const covered = new Set(result.edges.map((e) => e.to));
  const fromHand = new Set(await Promise.all(hand.filter((e) => e.from.includes('#') && covered.has(e.to)).map(key)));
  const both = [...fromMarkers].filter((k) => fromHand.has(k));
  console.log(`for the ${covered.size} requirements with markers: ${both.length} edges in both, ${fromMarkers.size - both.length} only from markers, ${fromHand.size - both.length} only by hand`);
  for (const k of fromMarkers) if (!fromHand.has(k)) console.log(`  only from markers: ${k}`);
  for (const k of fromHand) if (!fromMarkers.has(k)) console.log(`  only by hand: ${k}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
