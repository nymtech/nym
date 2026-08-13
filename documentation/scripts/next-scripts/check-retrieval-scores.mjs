#!/usr/bin/env node
/**
 * Print retrieval scores for a set of queries, to choose CHAT_MIN_SCORE.
 *
 * The floor decides when the assistant is handed no context and has to say a
 * topic is not covered. Too low and every question returns a full set of
 * "sources" however unrelated, so a refusal appears with ten citations beneath
 * it. Too high and real questions lose citations they should have had.
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

// Off-topic queries should score below the floor, on-topic ones above it. The
// gap between the two groups is what a good threshold sits in.
const OFF_TOPIC = [
  'What is the capital of France?',
  'Give me a recipe for carbonara',
  'Who won the 1998 world cup?',
];
const ON_TOPIC = [
  'What is a SURB?',
  'How do I run a nym-node?',
  'How do I use mix-fetch in a browser extension?',
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

/** Best and tenth-best score for one query, matching the route's topK of 10. */
async function scoreOf(query) {
  const vec = await embedQuery(query, provider);
  const scores = index.chunks
    .map((c) => cosine(vec, c.vector))
    .sort((a, b) => b - a)
    .slice(0, 10);
  return { query, top: scores[0] ?? 0, tenth: scores[9] ?? 0 };
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
    const { top, tenth } = await scoreOf(q);
    console.log(`  top=${top.toFixed(3)}  10th=${tenth.toFixed(3)}  ${q}`);
    if (label.startsWith('off')) seen.off.push(top);
    if (label.startsWith('on')) seen.on.push(tenth);
  }
}

if (seen.off.length && seen.on.length) {
  const highestOffTopic = Math.max(...seen.off);
  const lowestOnTopic = Math.min(...seen.on);
  console.log(
    `\nHighest off-topic score: ${highestOffTopic.toFixed(3)}` +
      `\nLowest on-topic score:  ${lowestOnTopic.toFixed(3)}`,
  );
  if (lowestOnTopic > highestOffTopic) {
    const suggested = (highestOffTopic + lowestOnTopic) / 2;
    console.log(`\nSet CHAT_MIN_SCORE around ${suggested.toFixed(2)} (midpoint of the gap).`);
  } else {
    console.log(
      '\nThe two groups overlap, so no threshold separates them cleanly. Favour the' +
        ' on-topic side and accept some irrelevant sources, or improve retrieval.',
    );
  }
}
