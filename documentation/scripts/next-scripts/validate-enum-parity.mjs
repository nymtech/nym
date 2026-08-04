// Enum-parity drift check.
//
// Several docs-facing TypeScript string unions mirror a Rust enum that is
// serialised over the wasm boundary with serde. If the two drift, the runtime
// emits a value the TS type (and the docs generated from it) says is impossible.
// This reads the Rust enum + its serde rename rule, derives the wire strings, and
// diffs them against the TS union. The TS union is the upstream of the generated
// docs, so this catches the doc drift at its source.
//
// Run (from documentation/docs; the script lives in the sibling documentation/scripts):
//   node ../scripts/next-scripts/validate-enum-parity.mjs            # check the pairs
//   node ../scripts/next-scripts/validate-enum-parity.mjs --selftest # fixtures only

import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url));

// The Rust enum / TS union pairs to keep in parity.
const CHECKS = [
  {
    label: 'TunnelState -> TunnelStateName',
    rustFile: 'wasm/smolmix/src/state.rs',
    rustEnum: 'TunnelState',
    tsFile: 'sdk/typescript/packages/mix-tunnel/src/types.ts',
    tsType: 'TunnelStateName',
  },
];

// PascalCase variant -> serde snake_case wire string ("ShuttingDown" -> "shutting_down").
export function toSnakeCase(variant) {
  return variant
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
    .toLowerCase();
}

// Slice the body between the matching braces of `enum NAME { ... }`.
function enumBody(text, enumName) {
  const head = text.match(new RegExp(`enum\\s+${enumName}\\s*\\{`));
  if (!head) throw new Error(`enum ${enumName} not found (source moved or renamed?)`);
  let i = head.index + head[0].length;
  let depth = 1;
  const start = i;
  for (; i < text.length && depth > 0; i++) {
    if (text[i] === '{') depth++;
    else if (text[i] === '}') depth--;
  }
  if (depth !== 0) throw new Error(`unbalanced braces in enum ${enumName}`);
  return text.slice(start, i - 1);
}

// The serde `rename_all` rule declared in the attributes just above the enum.
function serdeRename(text, enumName) {
  const at = text.search(new RegExp(`enum\\s+${enumName}\\b`));
  const before = text.slice(Math.max(0, at - 400), at);
  const m = before.match(/rename_all\s*=\s*"([^"]+)"/);
  return m ? m[1] : null;
}

// Variant names: split the body at top-level commas (those outside any variant's
// `{ ... }` or `( ... )` payload), then take the leading identifier of each chunk.
// Robust to one-per-line or comma-separated formatting.
export function rustEnumVariants(text, enumName) {
  const body = enumBody(text, enumName);
  const rule = serdeRename(text, enumName);
  if (rule && rule !== 'snake_case') {
    throw new Error(`unsupported serde rename_all "${rule}" on ${enumName}; extend the checker`);
  }
  const chunks = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < body.length; i++) {
    const ch = body[i];
    if (ch === '{' || ch === '(') depth++;
    else if (ch === '}' || ch === ')') depth--;
    else if (ch === ',' && depth === 0) {
      chunks.push(body.slice(start, i));
      start = i + 1;
    }
  }
  chunks.push(body.slice(start));
  const variants = chunks.map((c) => c.trim().match(/^([A-Z]\w*)/)).filter(Boolean).map((m) => m[1]);
  if (variants.length === 0) throw new Error(`no variants parsed from enum ${enumName}`);
  return variants.map((v) => (rule === 'snake_case' ? toSnakeCase(v) : v));
}

// String literals of `type NAME = 'a' | 'b' | ...;`.
export function tsUnionMembers(text, typeName) {
  const m = text.match(new RegExp(`type\\s+${typeName}\\s*=\\s*([^;]+);`));
  if (!m) throw new Error(`type ${typeName} not found (source moved or renamed?)`);
  const members = [...m[1].matchAll(/['"]([^'"]+)['"]/g)].map((x) => x[1]);
  if (members.length === 0) throw new Error(`no members parsed from union ${typeName}`);
  return members;
}

export function diffParity(expected, actual) {
  const e = new Set(expected);
  const a = new Set(actual);
  return {
    expected,
    actual,
    missingInTs: expected.filter((x) => !a.has(x)),
    extraInTs: actual.filter((x) => !e.has(x)),
  };
}

export function runCheck(check, repoRoot = REPO_ROOT) {
  const rust = readFileSync(join(repoRoot, check.rustFile), 'utf8');
  const ts = readFileSync(join(repoRoot, check.tsFile), 'utf8');
  const expected = rustEnumVariants(rust, check.rustEnum);
  const actual = tsUnionMembers(ts, check.tsType);
  return { ...check, ...diffParity(expected, actual) };
}

const SELFTEST = [
  {
    name: 'snake_case conversion of PascalCase variants',
    run: () => {
      const got = ['Connecting', 'Ready', 'ShuttingDown', 'Shutdown', 'Failed'].map(toSnakeCase);
      const want = ['connecting', 'ready', 'shutting_down', 'shutdown', 'failed'];
      return JSON.stringify(got) === JSON.stringify(want);
    },
  },
  {
    name: 'matched enum/union reports no drift',
    run: () => {
      const rust = '#[serde(tag = "s", rename_all = "snake_case")]\nenum S { Connecting, ShuttingDown, Failed { reason: R } }';
      const ts = "type T = 'connecting' | 'shutting_down' | 'failed';";
      const d = diffParity(rustEnumVariants(rust, 'S'), tsUnionMembers(ts, 'T'));
      return d.missingInTs.length === 0 && d.extraInTs.length === 0;
    },
  },
  {
    name: 'drifted union is flagged (the D1 shape)',
    run: () => {
      const rust = '#[serde(rename_all = "snake_case")]\nenum S { ShuttingDown, Shutdown }';
      const ts = "type T = 'disconnecting' | 'disconnected';";
      const d = diffParity(rustEnumVariants(rust, 'S'), tsUnionMembers(ts, 'T'));
      return (
        d.missingInTs.join() === 'shutting_down,shutdown' &&
        d.extraInTs.join() === 'disconnecting,disconnected'
      );
    },
  },
];

function runSelftest() {
  let failed = 0;
  for (const t of SELFTEST) {
    let pass = false;
    try {
      pass = t.run();
    } catch (e) {
      pass = false;
      console.log(`      ${e.message}`);
    }
    console.log(`${pass ? 'PASS' : 'FAIL'}  ${t.name}`);
    if (!pass) failed++;
  }
  return failed;
}

function runChecks() {
  let drift = 0;
  for (const check of CHECKS) {
    const r = runCheck(check);
    if (r.missingInTs.length === 0 && r.extraInTs.length === 0) {
      console.log(`OK    ${r.label}: ${r.actual.join(', ')}`);
      continue;
    }
    drift++;
    console.log(`DRIFT ${r.label}`);
    console.log(`      rust serialises: ${r.expected.join(', ')}  (${check.rustFile}:${check.rustEnum})`);
    console.log(`      ts declares:     ${r.actual.join(', ')}  (${check.tsFile}:${check.tsType})`);
    if (r.missingInTs.length) console.log(`      runtime emits but TS omits: ${r.missingInTs.join(', ')}`);
    if (r.extraInTs.length) console.log(`      TS claims but runtime never emits: ${r.extraInTs.join(', ')}`);
  }
  console.log(`\nChecked ${CHECKS.length} enum pair(s): ${drift} drift(s).`);
  return drift;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const run = process.argv.includes('--selftest') ? runSelftest : runChecks;
  process.exit(run() > 0 ? 1 : 0);
}
