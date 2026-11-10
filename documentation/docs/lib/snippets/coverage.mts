// Coverage report (D4c, task 6e.2): for each requirement record, the
// threat-model entities it links to and the records that realise it. A consumer
// that judges a statement needs to see where the graph is empty. For a requirement
// with a Tracey id, Tracey gives the code and the tests from their markers
// (add-spec-trace-gate, T1), so this report needs Tracey once a spec has ids.
//
//   node lib/snippets/coverage.mts [--list]

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { openRepo } from './repo.mts';
import { buildAll } from './records.mts';
import type { Snippet } from '../retrieval/types.ts';

export interface CoverageLine {
  id: string;
  spec: string;
  /** The threat-model ids the requirement links to. */
  threatModel: string[];
  /** The ids of the records that realise it, and the `file:line` of each `impl` marker. */
  realisedBy: string[];
  /** The `file:line` of each `verify` marker. */
  tests: string[];
}

type Markers = Map<string, { code: string[]; tests: string[] }>;

interface Ref { file: string; line: number }

/** The markers of one Tracey id. Tracey answers an id that its config does not hold with `{ "error": ... }`. */
function traceyRule(root: string, id: string): { code: string[]; tests: string[] } {
  const rule = JSON.parse(execFileSync('tracey', ['query', '--json', 'rule', id], { cwd: root, encoding: 'utf-8', stdio: ['ignore', 'pipe', 'pipe'] })) as { error?: string; coverage?: { implRefs: Ref[]; verifyRefs: Ref[] }[] };
  if (rule.error) throw new Error(`tracey: ${id}: ${rule.error}; add its spec to .config/tracey/config.styx`);
  const at = (refs: Ref[]): string[] => refs.map((r) => `${r.file}:${r.line}`);
  return { code: (rule.coverage ?? []).flatMap((c) => at(c.implRefs)), tests: (rule.coverage ?? []).flatMap((c) => at(c.verifyRefs)) };
}

type Records = Record<'requirement' | 'prose' | 'code' | 'model', Snippet[]>;

/** One line for each requirement. */
export function coverage({ requirement, prose, code, model }: Records, markers: Markers = new Map()): CoverageLine[] {
  const all = [...prose, ...requirement, ...code, ...model];
  return requirement.map((r) => ({
    id: r.id,
    spec: String(r.meta.spec),
    threatModel: r.refs.filter((x) => x.id.includes('/threat-model/')).map((x) => x.id),
    realisedBy: [...all.filter((o) => o.refs.some((x) => x.id === r.id && x.rel === 'realises')).map((o) => o.id), ...(markers.get(r.id)?.code ?? [])],
    tests: markers.get(r.id)?.tests ?? [],
  }));
}

async function main(argv: string[]): Promise<void> {
  const repo = openRepo();
  const built = await buildAll(repo);
  if (built.errors.length) throw new Error(`the records have errors; run records.mts:\n${built.errors.join('\n')}`);
  const markers: Markers = new Map();
  for (const r of built.requirement) {
    const tracey = r.meta.tracey as { id: string; version: number | null } | undefined;
    if (tracey) markers.set(r.id, traceyRule(repo.root, tracey.version ? `${tracey.id}+${tracey.version}` : tracey.id));
  }
  const lines = coverage(built, markers);
  const linked = lines.filter((l) => l.threatModel.length).length;
  const realised = lines.filter((l) => l.realisedBy.length).length;
  const chains = lines.filter((l) => l.threatModel.length && l.realisedBy.length).length;
  const tested = lines.filter((l) => l.tests.length).length;
  if (argv.includes('--list')) {
    for (const l of lines) console.log(`${l.id}  threat-model: ${l.threatModel.join(', ') || 'none'}  realised by: ${l.realisedBy.join(', ') || 'none'}  tests: ${l.tests.join(', ') || 'none'}`);
  }
  const bySpec = new Map<string, { n: number; linked: number }>();
  for (const l of lines) {
    const s = bySpec.get(l.spec) ?? { n: 0, linked: 0 };
    s.n++;
    if (l.threatModel.length) s.linked++;
    bySpec.set(l.spec, s);
  }
  console.log(`${lines.length} requirements: ${linked} link to a threat-model entity, ${realised} have a record that realises them, ${chains} complete chains (code -> requirement -> threat model), ${tested} have a test marker`);
  console.log(`specs with a link: ${[...bySpec].filter(([, s]) => s.linked).length} of ${bySpec.size}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
