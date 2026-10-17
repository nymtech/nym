// The runbook lint: check how-to and tutorial pages against their archetype.
//
//   node lib/archetypes/lint.mts <dir|file>...            check pages by their `diataxis` field
//   node lib/archetypes/lint.mts --assume how-to <dir>    check every page as that type (baseline counts)
//
// Prints each fault with file and line, a count for each rule, and the TSX
// components it could not check. Exits non-zero only on a fault from a rule at
// level `error` in its archetype file.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseFrontmatter } from '../retrieval/mdx.mjs';
import { openRepo } from '../snippets/repo.mjs';
import { inlinePage } from './inline.mts';
import type { InlinedPage } from './inline.mts';
import { fileOf, messageOf } from './mdast.mts';
import type { Archetype, Fault, LintWarning, Placeholder, PlaceholderIndex, Rule } from './types.mts';
import * as stepHeading from './rules/step-heading.mts';
import * as stepsPerProcedure from './rules/steps-per-procedure.mts';
import * as fenceLang from './rules/fence-lang.mts';
import * as fenceMeta from './rules/fence-meta.mts';
import * as placeholder from './rules/placeholder.mts';
import * as requiredSections from './rules/required-sections.mts';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const DOCS_ROOT = path.resolve(HERE, '../..');

export const RULES: Rule[] = [stepHeading, stepsPerProcedure, fenceLang, fenceMeta, placeholder, requiredSections];
/** The archetype file of each `diataxis` type that has one. */
const ARCHETYPE_OF = new Map([['how-to', 'runbook'], ['tutorial', 'tutorial']]);
const DIATAXIS = new Set(['tutorial', 'how-to', 'reference', 'explanation']);

/** Read a JSON file of this repository. Nothing checks the shape: `T` is the caller's statement of it. */
export function loadJson<T>(file: string): T {
  return JSON.parse(fs.readFileSync(file, 'utf-8'));
}

/** Index a placeholder list by every token it names: its `name` and each alias. */
export function indexPlaceholders(list: Placeholder[]): PlaceholderIndex {
  const byToken = new Map<string, Placeholder>();
  for (const entry of list) {
    if (!entry.name) continue;
    for (const token of [entry.name, ...(entry.aliases ?? [])]) {
      if (byToken.has(token)) throw new Error(`placeholders.json: ${token} is in two entries`);
      byToken.set(token, entry);
    }
  }
  return { byToken };
}

export interface LintOptions {
  root?: string;
  /** The directory of the archetype files and placeholders.json (tests pass a fixture directory). */
  dir?: string;
  /** Check the page as this type, whatever its `diataxis` field says. */
  assume?: string;
  placeholders?: PlaceholderIndex;
  aliases?: Record<string, string>;
}

export interface LintResult {
  type: string | null;
  faults: Fault[];
  unchecked: string[];
  warnings: LintWarning[];
}

/** Lint one page. */
export function lintPage(file: string, { root = DOCS_ROOT, dir = HERE, assume, placeholders, aliases = {} }: LintOptions = {}): LintResult {
  // The type first: a page with no archetype is not parsed at all.
  const { data: frontmatter } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
  // The frontmatter is untyped YAML: a number or a list gives the "unknown diataxis value" warning below.
  const value: unknown = assume ?? frontmatter.diataxis ?? null;
  const type = value === null ? null : String(value);
  const none: LintResult = { type, faults: [], unchecked: [], warnings: [] };
  if (type === null) return none;
  if (!DIATAXIS.has(type)) return { ...none, warnings: [{ file, line: 1, message: `unknown diataxis value "${type}"` }] };
  const archetypeName = ARCHETYPE_OF.get(type);
  if (!archetypeName) return none;

  let parsed: InlinedPage;
  try {
    // A split page imports its sections from the snippets: `aliases` lets the inliner follow them.
    parsed = inlinePage(file, { root, aliases });
  } catch (error) {
    return { ...none, warnings: [{ file, line: 1, message: `cannot parse: ${messageOf(error)}` }] };
  }
  const { tree, unchecked, warnings } = parsed;
  const archetype = loadJson<Archetype>(path.join(dir, `${archetypeName}.json`));
  const index = placeholders ?? indexPlaceholders(loadJson<Placeholder[]>(path.join(dir, 'placeholders.json')));
  const faults: Fault[] = [];
  for (const rule of RULES) {
    const level = archetype.rules[rule.id] ?? 'warn';
    if (level !== 'warn' && level !== 'error') throw new Error(`${archetypeName}.json: rule ${rule.id} has level "${level}"; use warn or error`);
    rule.check(tree, {
      report: (node, message, fix) => faults.push({
        rule: rule.id,
        level,
        file: fileOf(node) ?? file,
        line: node.position?.start.line ?? 1,
        message,
        fix,
      }),
      archetype,
      placeholders: index,
    });
  }
  return { type, faults, unchecked, warnings };
}

/** Every page file under a directory, skipping `_` files and dot entries. */
export function pageFiles(target: string): string[] {
  if (fs.statSync(target).isFile()) return [target];
  const files: string[] = [];
  for (const entry of fs.readdirSync(target, { withFileTypes: true })) {
    if (entry.name.startsWith('_') || entry.name.startsWith('.')) continue;
    const full = path.join(target, entry.name);
    if (entry.isDirectory()) files.push(...pageFiles(full));
    else if (/\.mdx?$/.test(entry.name)) files.push(full);
  }
  return files.sort();
}

function main(argv: string[]): void {
  const args = [...argv];
  let assume: string | undefined;
  const at = args.indexOf('--assume');
  if (at !== -1) {
    [, assume] = args.splice(at, 2);
    if (assume === undefined || !ARCHETYPE_OF.has(assume)) throw new Error(`--assume takes ${[...ARCHETYPE_OF.keys()].join(' or ')}`);
  }
  const targets = args.length > 0 ? args : ['content'];
  const placeholders = indexPlaceholders(loadJson<Placeholder[]>(path.join(HERE, 'placeholders.json')));
  // The snippet alias, read once: a split page imports its sections from the snippets.
  // repo.mjs is untyped until task 11.3 converts it.
  const aliases: Record<string, string> = openRepo().aliases;

  // A partial that two pages import gives the same fault twice; report it once.
  const faults = new Map<string, Fault>();
  const warnings = new Map<string, LintWarning>();
  const unchecked = new Map<string, string[]>();
  let checked = 0;
  let failedPages = 0;
  for (const file of targets.flatMap((t) => pageFiles(path.resolve(t)))) {
    const result = lintPage(file, { assume, placeholders, aliases });
    for (const w of result.warnings) warnings.set(`${w.file}:${w.line}:${w.message}`, w);
    if (result.type === null || !ARCHETYPE_OF.has(result.type)) continue;
    checked++;
    if (result.faults.length > 0) failedPages++;
    for (const f of result.faults) faults.set(`${f.file}:${f.line}:${f.rule}:${f.message}`, f);
    if (result.unchecked.length > 0) unchecked.set(file, result.unchecked);
  }

  const rel = (f: string) => path.relative(process.cwd(), f);
  for (const f of [...faults.values()].sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line)) {
    console.log(`${rel(f.file)}:${f.line}  ${f.level}  ${f.rule}  ${f.message}${f.fix ? ` (fix: ${f.fix})` : ''}`);
  }
  for (const w of warnings.values()) console.log(`${rel(w.file)}:${w.line}  warn  lint  ${w.message}`);

  if (unchecked.size > 0) {
    console.log('\nUnchecked TSX components (their content is not in the mdast):');
    for (const [file, names] of unchecked) console.log(`  ${rel(file)}: ${names.join(', ')}`);
  }

  const byRule = new Map(RULES.map((r) => [r.id, 0]));
  for (const f of faults.values()) byRule.set(f.rule, (byRule.get(f.rule) ?? 0) + 1);
  console.log(`\n${checked} pages checked, ${checked - failedPages} pass, ${failedPages} with faults`);
  for (const [rule, count] of byRule) console.log(`  ${rule.padEnd(20)} ${count}`);

  const errors = [...faults.values()].filter((f) => f.level === 'error').length;
  process.exitCode = errors > 0 ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
