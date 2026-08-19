#!/usr/bin/env node
/**
 * Print retrieval scores for a set of queries.
 *
 * Use it to watch the gap between on-topic and off-topic scores when the corpus,
 * the chunking or the embedding text changes. A gap that narrows means retrieval
 * got worse, whatever the absolute numbers do.
 *
 * Usage, from documentation/docs:
 *   VOYAGE_API_KEY=xxx node ../scripts/next-scripts/check-retrieval-scores.mjs
 *   VOYAGE_API_KEY=xxx node ../scripts/next-scripts/check-retrieval-scores.mjs "my own query"
 *
 * Reads public/docs-index.json, so build the index first.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { voyageProvider, embedQuery } from '../../docs/lib/retrieval/embed.mjs';

// Mirrors cosineSimilarity in lib/retrieval/retrieval.ts. Duplicated rather than
// imported because that module is TypeScript and this script runs under plain
// node; keep the two in step if the scoring there changes.
function cosine(a, b) {
  let dot = 0;
  let na = 0;
  let nb = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  const mag = Math.sqrt(na) * Math.sqrt(nb);
  return mag === 0 ? 0 : dot / mag;
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const INDEX = path.resolve(__dirname, '../../docs/public/docs-index.json');

// Queries with no answer in the corpus. They set the level the on-topic ones
// have to clear, and the distance between the two groups is the signal.
const OFF_TOPIC = [
  'What is the capital of France?',
  'Give me a recipe for carbonara',
  'Who won the 1998 world cup?',
];
// Two kinds of on-topic query, and the difference between them is the point. The
// well-formed ones score high. The short jargon ones are what developers actually
// type, and they score at or below the off-topic queries. The gap matters more
// than either absolute number. Keep both groups: dropping the second hides a
// failure the first cannot show.
const ON_TOPIC = [
  'What is a SURB?',
  'How do I run a nym-node?',
  'How do I use mix-fetch in a browser extension?',
  'Who is L2 and why does it matter?',
  'What does --use-anonymous-replies do?',
  'V1 vs V3',
];

const apiKey = process.env.VOYAGE_API_KEY;
if (!apiKey) {
  console.error('VOYAGE_API_KEY is required: queries have to be embedded to be scored.');
  process.exit(1);
}
if (!fs.existsSync(INDEX)) {
  console.error(`No index at ${INDEX}. Build the docs first.`);
  process.exit(1);
}

const index = JSON.parse(fs.readFileSync(INDEX, 'utf-8'));
if (!index.embedding?.dim) {
  console.error('Index has no vectors; it was built without VOYAGE_API_KEY.');
  process.exit(1);
}

const provider = voyageProvider({ apiKey });
const custom = process.argv.slice(2);

/** The ten best scores for one query. Wider than the MCP default topK of 6, so
 *  the weak tail stays visible. */
async function scoreOf(query) {
  const vec = await embedQuery(query, provider);
  const scores = index.chunks
    .map((c) => cosine(vec, c.vector))
    .sort((a, b) => b - a)
    .slice(0, 10);
  return { query, scores, top: scores[0] ?? 0, tenth: scores[9] ?? 0 };
}

const groups = custom.length
  ? [['custom', custom]]
  : [
      ['off-topic (want these below the floor)', OFF_TOPIC],
      ['on-topic (want these above it)', ON_TOPIC],
    ];

const seen = { off: [], on: [] };

for (const [label, queries] of groups) {
  console.log(`\n${label}`);
  for (const q of queries) {
    const result = await scoreOf(q);
    console.log(`  top=${result.top.toFixed(3)}  10th=${result.tenth.toFixed(3)}  ${q}`);
    if (label.startsWith('off')) seen.off.push(result);
    if (label.startsWith('on')) seen.on.push(result);
  }
}

if (seen.off.length && seen.on.length) {
  // A floor works if it silences every off-topic query while leaving each
  // on-topic one at least a couple of sources. It does NOT have to preserve all
  // ten: the tail of a top-10 is usually weak, and dropping it is the point.
  // Comparing against the on-topic 10th would reject thresholds that are fine.
  const floor = Math.max(...seen.off.map((r) => r.top));
  const survivors = (r, t) => r.scores.filter((s) => s >= t).length;

  console.log(`\nHighest off-topic score: ${floor.toFixed(3)} (the floor must clear this)`);

  const candidates = [];
  for (let t = Math.ceil(floor * 100) / 100; t <= 0.9; t += 0.01) {
    const counts = seen.on.map((r) => survivors(r, t));
    if (Math.min(...counts) < 2) break;
    candidates.push({ t, counts });
  }

  if (candidates.length === 0) {
    console.log(
      '\nNo threshold clears the off-topic queries while leaving every on-topic one at' +
        ' least 2 sources. That is the expected result. Nothing in the retrieval path' +
        ' applies a floor: search() accepts minScore but the MCP tools omit it, so the' +
        ' caller judges relevance from the top-K it gets back. Read the two groups above' +
        ' as a gap to watch, not a number to set.',
    );
  } else {
    console.log('\nSources kept per on-topic query, by threshold:');
    for (const c of candidates.slice(0, 6)) {
      console.log(`  ${c.t.toFixed(2)}  ->  ${c.counts.join(', ')}`);
    }
    console.log(
      '\nA threshold separates these particular queries, which earlier ones did not.' +
        ' Treat that as a sign retrieval improved, not as a value to set: the queries' +
        ' here are a sample, and the next short one is under no obligation to comply.',
    );
  }
}
