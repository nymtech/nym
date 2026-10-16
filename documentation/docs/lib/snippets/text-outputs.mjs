// Acceptance check for a docs split (D18 item 6, task 10.4.6): the text outputs
// of the docs site before and after a split must be the same.
//
//   node lib/snippets/text-outputs.mjs --save <dir>        run the three generators, keep the outputs
//   node lib/snippets/text-outputs.mjs --compare <a> <b>   compare two saved runs; exit 1 when they differ
//
// --save runs generate-llms-txt, generate-index, and generate-page-markdown with
// no embedding key and DOCS_PUBLIC_DIR set to <dir>/public, so public/ is not
// touched. It keeps llms-full.txt without its @generated line, the .md mirror,
// and the id, url, heading, and text of each chunk of docs-index.json. The
// Markdown outputs are compared in a normal form (see `normalise`).
// DOCS_CONTENT_DIR passes through, for a dry run on a copy of content/.

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { fenceTracker } from '../retrieval/mdx.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCRIPTS = path.resolve(HERE, '../../../scripts/next-scripts');
const GENERATORS = ['generate-llms-txt.mjs', 'generate-index.mjs', 'generate-page-markdown.mjs'];

/**
 * The compared form of a Markdown output, outside code fences only: a blank
 * line before and after each heading (a heading is its own block with or
 * without them), no trailing space on a heading (the anchor that split adds
 * replaces it), no blank line at the start, and no run of blank lines. Inside
 * a fence nothing changes. Both sides of a compare get it.
 */
function normalise(text) {
  const fence = fenceTracker();
  const out = [];
  let afterHeading = false;
  for (const line of text.split('\n')) {
    if (fence(line)) {
      if (afterHeading) out.push('');
      afterHeading = false;
      out.push(line);
      continue;
    }
    const heading = /^#{1,6} /.test(line);
    const blank = line.trim() === '';
    if ((heading || (afterHeading && !blank)) && out.length && out.at(-1).trim() !== '') out.push('');
    afterHeading = heading;
    if (blank && (out.length === 0 || out.at(-1).trim() === '')) continue;
    out.push(heading ? line.replace(/\s+$/, '') : line);
  }
  return out.join('\n');
}

function normaliseTree(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) normaliseTree(full);
    else if (e.name.endsWith('.md')) fs.writeFileSync(full, normalise(fs.readFileSync(full, 'utf-8')));
  }
}

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
  fs.writeFileSync(path.join(out, 'llms-full.txt'), normalise(llms));
  const { chunks } = JSON.parse(fs.readFileSync(path.join(pub, 'docs-index.json'), 'utf-8'));
  const kept = chunks.map(({ id, url, heading, text }) => ({ id, url, heading, text: normalise(text) })).sort((a, b) => (a.id < b.id ? -1 : 1));
  fs.writeFileSync(path.join(out, 'chunks.jsonl'), `${kept.map((c) => JSON.stringify(c)).join('\n')}\n`);
  fs.rmSync(path.join(pub, 'llms-full.txt'));
  fs.rmSync(path.join(pub, 'docs-index.json'));
  fs.renameSync(pub, path.join(out, 'mirror'));
  normaliseTree(path.join(out, 'mirror'));
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
