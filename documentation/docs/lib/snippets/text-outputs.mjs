// Acceptance check for a docs split (D18 item 6, task 10.4.6): the text outputs
// of the docs site before and after a split must be the same.
//
//   node lib/snippets/text-outputs.mjs --save <dir>        run the three generators, keep the outputs
//   node lib/snippets/text-outputs.mjs --compare <a> <b>   compare two saved runs; exit 1 when they differ
//
// --save runs generate-llms-txt, generate-index, and generate-page-markdown with
// no embedding key and DOCS_PUBLIC_DIR set to <dir>/public, so public/ is not
// touched. It keeps llms-full.txt without its @generated line, the .md mirror,
// and the id, url, heading, and text of each chunk of docs-index.json.
// DOCS_CONTENT_DIR passes through, for a dry run on a copy of content/.

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCRIPTS = path.resolve(HERE, '../../../scripts/next-scripts');
const GENERATORS = ['generate-llms-txt.mjs', 'generate-index.mjs', 'generate-page-markdown.mjs'];

function save(dir) {
  const out = path.resolve(dir);
  const pub = path.join(out, 'public');
  fs.rmSync(out, { recursive: true, force: true });
  fs.mkdirSync(pub, { recursive: true });
  const env = { ...process.env, DOCS_PUBLIC_DIR: pub };
  delete env.VOYAGE_API_KEY;
  delete env.VERCEL;
  delete env.REQUIRE_EMBEDDINGS;
  for (const g of GENERATORS) {
    const run = spawnSync(process.execPath, [path.join(SCRIPTS, g)], { cwd: path.resolve(HERE, '../..'), env, encoding: 'utf-8' });
    if (run.status !== 0) throw new Error(`${g} failed:\n${run.stderr || run.stdout}`);
  }
  const llms = fs.readFileSync(path.join(pub, 'llms-full.txt'), 'utf-8').replace(/^@generated: .*\n/m, '');
  fs.writeFileSync(path.join(out, 'llms-full.txt'), llms);
  const { chunks } = JSON.parse(fs.readFileSync(path.join(pub, 'docs-index.json'), 'utf-8'));
  const kept = chunks.map(({ id, url, heading, text }) => ({ id, url, heading, text })).sort((a, b) => (a.id < b.id ? -1 : 1));
  fs.writeFileSync(path.join(out, 'chunks.jsonl'), `${kept.map((c) => JSON.stringify(c)).join('\n')}\n`);
  fs.rmSync(path.join(pub, 'llms-full.txt'));
  fs.rmSync(path.join(pub, 'docs-index.json'));
  fs.renameSync(pub, path.join(out, 'mirror'));
  console.log(`saved ${kept.length} chunks, llms-full.txt, and the .md mirror to ${out}`);
}

function compare(a, b) {
  const diff = spawnSync('diff', ['-r', '-u', path.resolve(a), path.resolve(b)], { encoding: 'utf-8' });
  if (diff.status === 0) {
    console.log('text outputs identical');
    return 0;
  }
  process.stdout.write(diff.stdout);
  const files = (diff.stdout.match(/^diff -r -u /gm) ?? []).length + (diff.stdout.match(/^Only in /gm) ?? []).length;
  console.log(`text outputs differ in ${files} files`);
  return 1;
}

const argv = process.argv.slice(2);
if (argv[0] === '--save' && argv[1]) save(argv[1]);
else if (argv[0] === '--compare' && argv[1] && argv[2]) process.exitCode = compare(argv[1], argv[2]);
else throw new Error('usage: text-outputs.mjs --save <dir> | --compare <a> <b>');
