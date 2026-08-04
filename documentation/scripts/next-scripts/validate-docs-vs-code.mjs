// Docs-vs-code drift check (prototype).
//
// The docs assert falsifiable facts (constant values, sizes). Prose rots; the
// source is ground truth. This scans docs for size claims and cross-checks them
// against a small oracle of source-anchored constants, flagging contradictions.
//
// Scope of this prototype: numeric byte/KB size claims about the Sphinx packet
// format, matched by phrase pattern (the number tied to the noun it modifies,
// e.g. "2 KB payload", "348-byte routing header"), not by keyword proximity.
// The oracle values are DERIVED from source at run time (deriveSizeFacts): in-repo
// constants are read and evaluated, and the two external sphinx-packet leaves are
// pinned with a version check that fails loud if the crate is bumped. It reports
// drift candidates for review, it does not gate the build.
//
// Run (from documentation/docs; the script lives in the sibling documentation/scripts):
//   node ../scripts/next-scripts/validate-docs-vs-code.mjs              # scan the docs
//   node ../scripts/next-scripts/validate-docs-vs-code.mjs --selftest   # fixtures only
//   node ../scripts/next-scripts/validate-docs-vs-code.mjs --show-oracle # print derived facts

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url));

// The oracle's numbers are DERIVED from source, not hand-typed. In-repo constants
// are read and evaluated live (deriveConstants); the two leaves that live in the
// external sphinx-packet crate are pinned, but the pin self-invalidates if the
// crate version in the root Cargo.toml moves, so the oracle fails loud rather
// than validating against a stale value.

// The only pinned values: sphinx-packet is a registry dependency, so its consts
// are not in this tree. Verified against the version below; deriveConstants
// throws if Cargo.toml no longer pins that version.
const SPHINX_PACKET_VERSION = '0.6.0';
const EXTERNAL_PINS = {
  HEADER_SIZE: 348, // sphinx-packet header::HEADER_SIZE
  PAYLOAD_OVERHEAD_SIZE: 17, // sphinx-packet PAYLOAD_OVERHEAD_SIZE
};

// In-repo Rust constants to read and compose. Each is `const NAME: usize = <expr>`
// where <expr> is arithmetic over numbers and other (in-repo or pinned) constants.
const IN_REPO_CONSTS = [
  {
    file: 'common/nymsphinx/params/src/packet_sizes.rs',
    names: ['SPHINX_PACKET_OVERHEAD', 'REGULAR_PACKET_SIZE'],
  },
];

function readConst(text, name) {
  const m = text.match(new RegExp(`\\bconst\\s+${name}\\s*:\\s*usize\\s*=\\s*([^;]+);`));
  if (!m) throw new Error(`constant ${name} not found (source moved or renamed?)`);
  return m[1].trim();
}

// Evaluate a Rust integer const expression over +, *, parens, numbers and known
// identifiers. Recursive-descent; a `resolve` callback supplies identifier values.
function evalExpr(expr, resolve) {
  const tokens = expr.match(/\d+|[A-Za-z_][A-Za-z0-9_]*|[+*()]/g) || [];
  let i = 0;
  const peek = () => tokens[i];
  const sum = () => {
    let v = product();
    while (peek() === '+') { i++; v += product(); }
    return v;
  };
  const product = () => {
    let v = factor();
    while (peek() === '*') { i++; v *= factor(); }
    return v;
  };
  const factor = () => {
    const t = tokens[i++];
    if (t === '(') { const v = sum(); if (tokens[i++] !== ')') throw new Error(`unbalanced parens in "${expr}"`); return v; }
    if (/^\d+$/.test(t)) return parseInt(t, 10);
    if (/^[A-Za-z_]/.test(t)) return resolve(t);
    throw new Error(`unexpected token "${t}" in "${expr}"`);
  };
  const v = sum();
  if (i !== tokens.length) throw new Error(`trailing tokens in "${expr}"`);
  return v;
}

// Resolve the named constants from source into concrete integers.
export function deriveConstants(repoRoot = REPO_ROOT) {
  const cargo = readFileSync(join(repoRoot, 'Cargo.toml'), 'utf8');
  const ver = cargo.match(/^sphinx-packet\s*=\s*"=?([\d.]+)"/m);
  if (!ver) throw new Error('could not find the sphinx-packet pin in Cargo.toml');
  if (ver[1] !== SPHINX_PACKET_VERSION) {
    throw new Error(
      `sphinx-packet is now ${ver[1]}, pins were verified against ${SPHINX_PACKET_VERSION}; ` +
        're-verify HEADER_SIZE / PAYLOAD_OVERHEAD_SIZE and update EXTERNAL_PINS',
    );
  }

  const exprs = {}; // name -> unresolved RHS expression string
  for (const { file, names } of IN_REPO_CONSTS) {
    const text = readFileSync(join(repoRoot, file), 'utf8');
    for (const n of names) exprs[n] = readConst(text, n);
  }

  const cache = { ...EXTERNAL_PINS };
  const inProgress = new Set();
  const resolve = (name) => {
    if (name in cache) return cache[name];
    if (!(name in exprs)) throw new Error(`unresolved identifier ${name} (not a pin or a read const)`);
    if (inProgress.has(name)) throw new Error(`cyclic constant ${name}`);
    inProgress.add(name);
    const v = evalExpr(exprs[name], resolve);
    inProgress.delete(name);
    cache[name] = v;
    return v;
  };

  return {
    HEADER_SIZE: resolve('HEADER_SIZE'),
    PAYLOAD_OVERHEAD_SIZE: resolve('PAYLOAD_OVERHEAD_SIZE'),
    SPHINX_PACKET_OVERHEAD: resolve('SPHINX_PACKET_OVERHEAD'),
    REGULAR_PACKET_SIZE: resolve('REGULAR_PACKET_SIZE'),
  };
}

// Fact shape is static (id, the noun phrases to match, the required context); the
// byte value is derived from the resolved constants. `context` must appear in the
// same sentence as a bound number for the claim to count: nouns like "payload"
// and "byte" are common, so gating on "sphinx" keeps the check to the packet
// format and avoids firing on unrelated framing (LP frames, WireGuard MTU).
const FACT_SPECS = [
  {
    id: 'sphinx-packet-size',
    nouns: ['sphinx packet', 'sphinx packets'],
    context: /sphinx/i,
    bytes: (c) => c.REGULAR_PACKET_SIZE,
    source: 'common/nymsphinx/params/src/packet_sizes.rs REGULAR_PACKET_SIZE',
  },
  {
    // Longer noun than the bare "payload" below; findNouns matches longest-first,
    // so "payload overhead" claims its span before "payload" can.
    id: 'sphinx-payload-overhead',
    nouns: ['payload overhead'],
    context: /sphinx/i,
    bytes: (c) => c.PAYLOAD_OVERHEAD_SIZE,
    source: `sphinx-packet ${SPHINX_PACKET_VERSION} PAYLOAD_OVERHEAD_SIZE`,
  },
  {
    id: 'sphinx-payload-size',
    nouns: ['payload'],
    context: /sphinx/i,
    // plaintext_size() = size - header - payload_overhead
    bytes: (c) => c.REGULAR_PACKET_SIZE - c.HEADER_SIZE - c.PAYLOAD_OVERHEAD_SIZE,
    source: 'common/nymsphinx PacketSize::plaintext_size() = size - header - payload_overhead',
  },
  {
    id: 'sphinx-header-size',
    nouns: ['routing header', 'sphinx header'],
    context: /sphinx/i,
    bytes: (c) => c.HEADER_SIZE,
    source: `sphinx-packet ${SPHINX_PACKET_VERSION} header::HEADER_SIZE`,
  },
];

// Build the oracle by resolving each fact's byte value from source.
export function deriveSizeFacts(repoRoot = REPO_ROOT) {
  const c = deriveConstants(repoRoot);
  return FACT_SPECS.map((s) => ({
    id: s.id,
    nouns: s.nouns,
    context: s.context,
    bytes: s.bytes(c),
    source: `${s.source} = ${s.bytes(c)} (derived)`,
  }));
}

export const SIZE_FACTS = deriveSizeFacts();

const UNIT_BYTES = { b: 1, byte: 1, bytes: 1, kb: 1024, kib: 1024 };

// A number bound to a unit. The unit may follow a hyphen ("348-byte") or a
// space ("2 KB"), so both are allowed.
const NUM_RE = /(\d[\d,.]*)[\s-]?(bytes?|kib|kb|b)\b/gi;

function toBytes(numText, unit) {
  const n = parseFloat(numText.replace(/,/g, ''));
  const mult = UNIT_BYTES[unit.toLowerCase()];
  return Number.isFinite(n) && mult ? Math.round(n * mult) : null;
}

function findNumbers(text) {
  const out = [];
  for (const m of text.matchAll(NUM_RE)) {
    const bytes = toBytes(m[1], m[2]);
    if (bytes !== null) out.push({ start: m.index, end: m.index + m[0].length, bytes });
  }
  return out;
}

// Locate every fact-noun occurrence, longest noun first so "routing header"
// claims the span before the bare "header" alias can, and overlaps are skipped.
function findNouns(lower, facts) {
  const entries = facts
    .flatMap((f) => f.nouns.map((n) => ({ text: n.toLowerCase(), fact: f })))
    .sort((a, b) => b.text.length - a.text.length);
  const claimed = [];
  const out = [];
  for (const { text, fact } of entries) {
    let from = 0;
    for (;;) {
      const i = lower.indexOf(text, from);
      if (i < 0) break;
      const end = i + text.length;
      if (!claimed.some((r) => i < r.end && end > r.start)) {
        claimed.push({ start: i, end });
        out.push({ start: i, end, fact });
      }
      from = i + 1;
    }
  }
  return out;
}

// Bind each number to the noun it actually modifies, then compare to the oracle.
//   1. tight-forward: a number followed by a noun with only adjective-like words
//      between them ("2 KB payload", "348-byte per-hop routing header") binds to
//      it and consumes that noun. The gap must be letters/spaces/hyphens only,
//      so an intervening number (a different clause) breaks the bond.
//   2. nearest-preceding: a leftover number binds to the closest earlier unused
//      noun within a short window ("packet is a constant 2000 bytes").
// A number with no noun in reach is left unclassified (not reported), so stray
// numbers elsewhere on the page do not produce false positives.
const FORWARD_WORD_GAP = 24;
const BACKWARD_WINDOW = 30;

// The sentence (bounded by `.`, newline, or block edge) surrounding an index,
// used to test a fact's required context.
function sentenceAround(text, index) {
  let start = index;
  while (start > 0 && !'.\n'.includes(text[start - 1])) start--;
  let end = index;
  while (end < text.length && !'.\n'.includes(text[end])) end++;
  return text.slice(start, end);
}

export function analyseText(text, facts = SIZE_FACTS) {
  const lower = text.toLowerCase();
  const numbers = findNumbers(text);
  const nouns = findNouns(lower, facts);
  const used = new Set();
  const results = [];

  const bind = (num, noun) => {
    if (noun.fact.context && !noun.fact.context.test(sentenceAround(text, num.start))) return;
    used.add(noun);
    results.push({
      factId: noun.fact.id,
      expectedBytes: noun.fact.bytes,
      claimedBytes: num.bytes,
      claimedText: text.slice(num.start, noun.end > num.end ? noun.end : num.end).trim(),
      status: num.bytes === noun.fact.bytes ? 'ok' : 'drift',
      source: noun.fact.source,
    });
  };

  const pending = [];
  for (const num of numbers) {
    const fwd = nouns.find((n) => {
      if (used.has(n) || n.start < num.end) return false;
      const gap = text.slice(num.end, n.start);
      return gap.length <= FORWARD_WORD_GAP && /^[A-Za-z\s-]*$/.test(gap);
    });
    if (fwd) bind(num, fwd);
    else pending.push(num);
  }
  for (const num of pending) {
    const candidates = nouns
      .filter((n) => !used.has(n) && n.end <= num.start && num.start - n.end <= BACKWARD_WINDOW)
      .sort((a, b) => b.end - a.end);
    if (candidates.length) bind(num, candidates[0]);
  }
  return results;
}

function walk(dir, exts, out = []) {
  for (const entry of readdirSync(dir)) {
    if (entry === 'node_modules' || entry === '.next' || entry.startsWith('.')) continue;
    const p = join(dir, entry);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, exts, out);
    else if (exts.some((e) => entry.endsWith(e))) out.push(p);
  }
  return out;
}

const SELFTEST_FIXTURES = [
  {
    name: 'drifted packet size (the bug this was built to catch)',
    text: 'Every Sphinx packet is a constant 2000 bytes. It holds a routing header and a payload.',
    expect: { drift: 1 },
  },
  {
    name: 'corrected dense sentence must not false-positive',
    text: 'Each Sphinx packet is a constant 2413 bytes: a 2 KB payload behind a 348-byte routing header.',
    expect: { drift: 0, ok: 3 },
  },
  {
    name: 'agreeing payload claim on another page passes',
    text: 'All Sphinx packets have a fixed payload size of 2048 bytes.',
    expect: { drift: 0, ok: 1 },
  },
];

function runSelftest() {
  let failed = 0;
  for (const fx of SELFTEST_FIXTURES) {
    const claims = analyseText(fx.text);
    const got = { ok: claims.filter((c) => c.status === 'ok').length, drift: claims.filter((c) => c.status === 'drift').length };
    const pass =
      (fx.expect.drift === undefined || got.drift === fx.expect.drift) &&
      (fx.expect.ok === undefined || got.ok === fx.expect.ok);
    console.log(`${pass ? 'PASS' : 'FAIL'}  ${fx.name}`);
    if (!pass) {
      failed++;
      console.log(`      expected ${JSON.stringify(fx.expect)}, got ${JSON.stringify(got)}`);
      console.log(`      claims: ${JSON.stringify(claims, null, 2)}`);
    }
  }
  return failed;
}

function runScan() {
  const here = fileURLToPath(new URL('.', import.meta.url));
  const docsRoot = join(here, '..', '..', 'docs');
  const roots = [join(docsRoot, 'pages'), join(docsRoot, 'lib', 'privacy-model')];
  const files = roots.flatMap((r) => {
    try {
      return walk(r, ['.md', '.mdx', '.ts']);
    } catch {
      return [];
    }
  });

  let drift = 0;
  let ok = 0;
  for (const file of files) {
    const claims = analyseText(readFileSync(file, 'utf8'));
    for (const c of claims) {
      if (c.status === 'ok') {
        ok++;
        continue;
      }
      drift++;
      console.log(`DRIFT  ${relative(docsRoot, file)}`);
      console.log(`       claim:    "${c.claimedText}" = ${c.claimedBytes} B`);
      console.log(`       expected: ${c.expectedBytes} B  (${c.factId})`);
      console.log(`       source:   ${c.source}`);
    }
  }
  console.log(`\nScanned ${files.length} files: ${ok} claim(s) agree, ${drift} drift candidate(s).`);
  return drift;
}

function runShowOracle() {
  const c = deriveConstants();
  console.log('Constants derived from source:');
  for (const [k, v] of Object.entries(c)) console.log(`  ${k} = ${v}`);
  console.log('\nOracle facts:');
  for (const f of SIZE_FACTS) console.log(`  ${f.id} = ${f.bytes} B  (${f.source})`);
  return 0;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  let run = runScan;
  if (process.argv.includes('--selftest')) run = runSelftest;
  else if (process.argv.includes('--show-oracle')) run = runShowOracle;
  process.exit(run() > 0 ? 1 : 0);
}
