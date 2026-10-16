// Coverage report (D4c, task 6e.2): for each requirement record, the
// threat-model entities it links to and the records that realise it. A consumer
// that judges a statement needs to see where the graph is empty.
//
//   node lib/snippets/coverage.mjs [--list]

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { openRepo } from './repo.mjs';
import { buildAll } from './records.mjs';

/** One line for each requirement: { id, spec, threatModel: ids, realisedBy: ids }. */
export function coverage({ requirement, prose, code, model }) {
  const all = [...prose, ...requirement, ...code, ...model];
  return requirement.map((r) => ({
    id: r.id,
    spec: r.meta.spec,
    threatModel: r.refs.filter((x) => x.id.includes('/threat-model/')).map((x) => x.id),
    realisedBy: all.filter((o) => o.refs.some((x) => x.id === r.id && x.rel === 'realises')).map((o) => o.id),
  }));
}

async function main(argv) {
  const built = await buildAll(openRepo());
  if (built.errors.length) throw new Error(`the records have errors; run records.mjs:\n${built.errors.join('\n')}`);
  const lines = coverage(built);
  const linked = lines.filter((l) => l.threatModel.length).length;
  const realised = lines.filter((l) => l.realisedBy.length).length;
  const chains = lines.filter((l) => l.threatModel.length && l.realisedBy.length).length;
  if (argv.includes('--list')) {
    for (const l of lines) console.log(`${l.id}  threat-model: ${l.threatModel.join(', ') || 'none'}  realised by: ${l.realisedBy.join(', ') || 'none'}`);
  }
  const bySpec = new Map();
  for (const l of lines) {
    const s = bySpec.get(l.spec) ?? { n: 0, linked: 0 };
    s.n++;
    if (l.threatModel.length) s.linked++;
    bySpec.set(l.spec, s);
  }
  console.log(`${lines.length} requirements: ${linked} link to a threat-model entity, ${realised} have a record that realises them, ${chains} complete chains (code -> requirement -> threat model)`);
  console.log(`specs with a link: ${[...bySpec].filter(([, s]) => s.linked).length} of ${bySpec.size}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
