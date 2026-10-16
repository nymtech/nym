// The runbook lint: check how-to and tutorial pages against their archetype.
//
//   node lib/archetypes/lint.mjs <dir|file>...            check pages by their `diataxis` field
//   node lib/archetypes/lint.mjs --assume how-to <dir>    check every page as that type (baseline counts)
//
// Prints each fault with file and line, a count for each rule, and the TSX
// components it could not check. Exits non-zero only on a fault from a rule at
// level `error` in its archetype file.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { inlinePage } from './inline.mjs';
import { parseFrontmatter } from '../retrieval/mdx.mjs';
import { openRepo } from '../snippets/repo.mjs';
import * as stepHeading from './rules/step-heading.mjs';
import * as stepsPerProcedure from './rules/steps-per-procedure.mjs';
import * as fenceLang from './rules/fence-lang.mjs';
import * as fenceMeta from './rules/fence-meta.mjs';
import * as placeholder from './rules/placeholder.mjs';
import * as requiredSections from './rules/required-sections.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const DOCS_ROOT = path.resolve(HERE, '../..');

export const RULES = [stepHeading, stepsPerProcedure, fenceLang, fenceMeta, placeholder, requiredSections];
const ARCHETYPE_OF = { 'how-to': 'runbook', tutorial: 'tutorial' };
const DIATAXIS = new Set(['tutorial', 'how-to', 'reference', 'explanation']);

export function loadJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf-8'));
}

/** Index a placeholder list by every token it names: its `name` and each alias. */
export function indexPlaceholders(list) {
  const byToken = new Map();
  for (const entry of list) {
    if (!entry.name) continue;
    for (const token of [entry.name, ...(entry.aliases ?? [])]) {
      if (byToken.has(token)) throw new Error(`placeholders.json: ${token} is in two entries`);
      byToken.set(token, entry);
    }
  }
  return { byToken };
}

/**
 * Lint one page.
 * @param {string} file
 * @param {{ root?: string, dir?: string, assume?: string, placeholders?: object }} opts
 *   `dir` holds the archetype files and placeholders.json (tests pass a fixture dir).
 * @returns {{ type: string|null, faults: object[], unchecked: string[], warnings: object[] }}
 */
export function lintPage(file, { root = DOCS_ROOT, dir = HERE, assume, placeholders, aliases = {} } = {}) {
  // The type first: a page with no archetype is not parsed at all.
  const { data: frontmatter } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
  const type = assume ?? frontmatter.diataxis ?? null;
  const none = { type, faults: [], unchecked: [], warnings: [] };
  if (type === null) return none;
  if (!DIATAXIS.has(type)) return { ...none, warnings: [{ file, line: 1, message: `unknown diataxis value "${type}"` }] };
  const archetypeName = ARCHETYPE_OF[type];
  if (!archetypeName) return none;

  let parsed;
  try {
    // A split page imports its sections from the snippets: `aliases` lets the inliner follow them.
    parsed = inlinePage(file, { root, aliases });
  } catch (error) {
    return { ...none, warnings: [{ file, line: 1, message: `cannot parse: ${error.message}` }] };
  }
  const { tree, unchecked, warnings } = parsed;
  const archetype = loadJson(path.join(dir, `${archetypeName}.json`));
  const index = placeholders ?? indexPlaceholders(loadJson(path.join(dir, 'placeholders.json')));
  const faults = [];
  for (const rule of RULES) {
    const level = archetype.rules[rule.id] ?? 'warn';
    if (level !== 'warn' && level !== 'error') throw new Error(`${archetypeName}.json: rule ${rule.id} has level "${level}"; use warn or error`);
    const report = (node, message, fix) => faults.push({
      rule: rule.id,
      level,
      file: node.data?.file ?? file,
      line: node.position?.start.line ?? 1,
      message,
      fix,
    });
    rule.check(tree, { report, archetype, placeholders: index });
  }
  return { type, faults, unchecked, warnings };
}

/** Every page file under a directory, skipping `_` files and dot entries. */
export function pageFiles(target) {
  if (fs.statSync(target).isFile()) return [target];
  const files = [];
  for (const entry of fs.readdirSync(target, { withFileTypes: true })) {
    if (entry.name.startsWith('_') || entry.name.startsWith('.')) continue;
    const full = path.join(target, entry.name);
    if (entry.isDirectory()) files.push(...pageFiles(full));
    else if (/\.mdx?$/.test(entry.name)) files.push(full);
  }
  return files.sort();
}

function main(argv) {
  const args = [...argv];
  let assume;
  const at = args.indexOf('--assume');
  if (at !== -1) {
    [, assume] = args.splice(at, 2);
    if (!ARCHETYPE_OF[assume]) throw new Error(`--assume takes ${Object.keys(ARCHETYPE_OF).join(' or ')}`);
  }
  const targets = args.length > 0 ? args : ['content'];
  const placeholders = indexPlaceholders(loadJson(path.join(HERE, 'placeholders.json')));
  // The snippet alias, read once: a split page imports its sections from the snippets.
  const { aliases } = openRepo();

  // A partial that two pages import gives the same fault twice; report it once.
  const faults = new Map();
  const warnings = new Map();
  const unchecked = new Map();
  let checked = 0;
  let failedPages = 0;
  for (const file of targets.flatMap((t) => pageFiles(path.resolve(t)))) {
    const result = lintPage(file, { assume, placeholders, aliases });
    for (const w of result.warnings) warnings.set(`${w.file}:${w.line}:${w.message}`, w);
    if (!ARCHETYPE_OF[result.type]) continue;
    checked++;
    if (result.faults.length > 0) failedPages++;
    for (const f of result.faults) faults.set(`${f.file}:${f.line}:${f.rule}:${f.message}`, f);
    if (result.unchecked.length > 0) unchecked.set(file, result.unchecked);
  }

  const rel = (f) => path.relative(process.cwd(), f);
  for (const f of [...faults.values()].sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line)) {
    console.log(`${rel(f.file)}:${f.line}  ${f.level}  ${f.rule}  ${f.message}${f.fix ? ` (fix: ${f.fix})` : ''}`);
  }
  for (const w of warnings.values()) console.log(`${rel(w.file)}:${w.line}  warn  lint  ${w.message}`);

  if (unchecked.size > 0) {
    console.log('\nUnchecked TSX components (their content is not in the mdast):');
    for (const [file, names] of unchecked) console.log(`  ${rel(file)}: ${names.join(', ')}`);
  }

  const byRule = new Map(RULES.map((r) => [r.id, 0]));
  for (const f of faults.values()) byRule.set(f.rule, byRule.get(f.rule) + 1);
  console.log(`\n${checked} pages checked, ${checked - failedPages} pass, ${failedPages} with faults`);
  for (const [rule, count] of byRule) console.log(`  ${rule.padEnd(20)} ${count}`);

  const errors = [...faults.values()].filter((f) => f.level === 'error').length;
  process.exitCode = errors > 0 ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
