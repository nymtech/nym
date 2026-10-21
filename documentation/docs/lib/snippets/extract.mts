// Code extractor (D5, D6): a citation is the request to extract. Scan each prose
// snippet for `<Snippet id="<repo>/<path>#<symbol>" />`, find the item in the
// source file with tree-sitter, and build one `code` record for it.
//
// A symbol that is missing, or that names more than one item, is an error that
// names the snippet file and line of the citation: the drift check.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { CITATION, fenceTracker } from '../retrieval/mdx.mjs';
import { items, matchSymbol } from '../retrieval/items.mjs';
import type { Snippet } from '../retrieval/types.ts';
import type { Repo } from './repo.mts';
import { loadEdges } from './edges.mts';

const LANGS: Record<string, string> = { '.rs': 'rust' };

/** An item of a source file, as items.mjs returns it. */
interface Item {
  name: string;
  qualifiedName: string;
  kind: string;
  startLine: number;
  endLine: number;
}

export interface CodeCitation {
  id: string;
  file: string;
  line: number;
}

/** A code record before the record builder gives it its authority. */
export type CodeRecord = Omit<Snippet, 'authority'>;

export interface Extracted {
  /** Id -> record. */
  code: Map<string, CodeRecord>;
  /** The ids that could not be extracted. */
  failed: Set<string>;
  errors: string[];
}

/** Each code citation of this repository in the prose snippets. */
export function codeCitations(repo: Repo): CodeCitation[] {
  const found: CodeCitation[] = [];
  const walk = (dir: string): void => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.mdx')) scan(full);
    }
  };
  const scan = (file: string): void => {
    const lines = fs.readFileSync(file, 'utf-8').split('\n');
    // Skip the frontmatter, but keep the file's own line numbers.
    let i = 0;
    if (lines[0] === '---') i = lines.indexOf('---', 1) + 1;
    const fence = fenceTracker();
    for (; i < lines.length; i++) {
      const line = lines[i] ?? '';
      if (fence(line)) continue;
      for (const [, id = ''] of line.matchAll(CITATION)) {
        if (id.includes('#') && id.startsWith(`${repo.repo}/`)) found.push({ id, file, line: i + 1 });
      }
    }
  };
  if (fs.existsSync(repo.snippets)) walk(repo.snippets);
  // A code item that an authored edge starts from is a request to extract, too.
  const edgesFile = path.join(repo.dir, 'edges.json');
  loadEdges(repo).forEach((e, i) => {
    if (typeof e.from === 'string' && e.from.includes('#') && e.from.startsWith(`${repo.repo}/`)) found.push({ id: e.from, file: edgesFile, line: i + 1 });
  });
  const cmp = (x: string, y: string): number => (x < y ? -1 : x > y ? 1 : 0);
  return found.sort((a, b) => cmp(a.id, b.id) || cmp(a.file, b.file) || a.line - b.line);
}

/** Extract every cited code item. */
export async function extractCode(repo: Repo): Promise<Extracted> {
  const code = new Map<string, CodeRecord>();
  const failed = new Set<string>();
  const errors: string[] = [];
  const where = (c: CodeCitation): string => `${path.relative(repo.root, c.file)}:${c.line}`;
  const fail = (c: CodeCitation, message: string): void => { failed.add(c.id); errors.push(`${where(c)}: ${c.id}: ${message}`); };
  // Source path -> its text and items, so each file is read and parsed once.
  const parsed = new Map<string, { text: string; list: Item[] }>();

  for (const c of codeCitations(repo)) {
    if (code.has(c.id)) continue; // each failing citation is reported, not only the first
    const hash = c.id.indexOf('#');
    const source = c.id.slice(repo.repo.length + 1, hash);
    const symbol = c.id.slice(hash + 1);
    const lang = LANGS[path.extname(source)];
    const file = path.join(repo.root, source);
    if (!lang) {
      fail(c, `no extractor for ${path.extname(source) || 'a file with no extension'}`);
      continue;
    }
    if (!fs.existsSync(file)) {
      fail(c, `${source} does not exist`);
      continue;
    }
    let entry = parsed.get(source);
    if (!entry) {
      const text = fs.readFileSync(file, 'utf-8');
      entry = { text, list: await items(text, lang) };
      parsed.set(source, entry);
    }
    const { text, list } = entry;
    const matches: Item[] = matchSymbol(list, symbol);
    const [item, ...others] = matches;
    if (!item) {
      fail(c, `no item ${symbol} in ${source}`);
      continue;
    }
    if (others.length > 0) {
      const list = matches.map((m) => `${m.qualifiedName} (line ${m.startLine})`).join(', ');
      fail(c, `${symbol} names ${matches.length} items; qualify it: ${list}`);
      continue;
    }
    const body = text.split('\n').slice(item.startLine - 1, item.endLine).join('\n');
    code.set(c.id, {
      id: c.id,
      kind: 'code',
      repo: repo.repo,
      path: source,
      lines: [item.startLine, item.endLine],
      title: symbol,
      lang,
      body,
      hash: crypto.createHash('sha256').update(body).digest('hex'),
      refs: [],
      meta: { qualifiedName: item.qualifiedName, kind: item.kind },
    });
  }
  return { code, failed, errors };
}
