// Round trip (D10): compose each split page back into one text and compare it
// with the page it came from.
//
//   node lib/snippets/roundtrip.mjs --site codex --from <original content dir> [--dir <page dir>] [--root <repo root>]
//   node lib/snippets/roundtrip.mjs --site codex --print <page file under the page root>
//
// --print writes one composed page to stdout, for a writer or a reviewer who
// wants to read the page as one text (task 5.1).
//
// Compose puts the text of each snippet in place of its import, each diagram
// back as a `mermaid` fence, and each `snippet:` link back as its route. The
// compare ignores whitespace runs and the explicit heading anchors that `split`
// adds. Exit code 1 when a page differs.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { inlineMdxPartials, stripHeadingIds } from '../retrieval/mdx.mjs';
import { splitFrontmatter } from './split.mjs';
import { openRepo, citationText, pageLinks, linkRoute, pageFiles, routeOf } from './repo.mjs';
import { extractCode } from './extract.mjs';

/** The page as one text: frontmatter, then the composed body with no snippet imports. */
export function composePage(repo, site, file, links = pageLinks(repo, site), code = new Map()) {
  const { frontmatter, rest } = splitFrontmatter(fs.readFileSync(file, 'utf-8'));
  const page = routeOf(path.relative(path.join(repo.root, repo.manifest.sites[site].pages), file));
  const body = inlineMdxPartials(rest, {
    filePath: file,
    root: repo.root,
    lineOffset: frontmatter.split('\n').length - 1,
    aliases: repo.aliases,
    citation: (id) => citationText(repo, id, { code }),
    link: (id) => linkRoute(links, id, page),
  });
  return frontmatter + body.split('\n').filter((l) => !/^import\s+\w+\s+from\s+['"]@snippets\//.test(l)).join('\n');
}

/**
 * The form both sides are compared in. An ATX heading starts a new block with
 * or without a blank line above it (CommonMark), so a blank line is put before
 * each heading on both sides.
 */
export function normalise(text) {
  return stripHeadingIds(text)
    .split('\n')
    .map((l) => l.replace(/\s+$/, ''))
    .join('\n')
    .replace(/\n(?=#{1,6} )/g, '\n\n')
    .replace(/\n{2,}/g, '\n\n')
    .trim();
}

async function main(argv) {
  const opt = (name, fallback) => {
    const at = argv.indexOf(`--${name}`);
    if (at !== -1) return argv[at + 1];
    if (fallback === undefined) throw new Error(`--${name} is required`);
    return fallback;
  };
  const rootArg = opt('root', null);
  const repo = openRepo(rootArg ? path.resolve(rootArg) : undefined);
  const site = opt('site');
  const print = opt('print', null);
  if (print) {
    const { code, errors } = await extractCode(repo);
    if (errors.length) throw new Error(`extraction failed:\n${errors.join('\n')}`);
    const text = composePage(repo, site, path.join(repo.root, repo.manifest.sites[site].pages, print), pageLinks(repo, site), code);
    process.stdout.write(`${text.replace(/\n{3,}/g, '\n\n').trim()}\n`);
    return;
  }
  const from = path.resolve(opt('from'));
  const dir = opt('dir', '').replace(/\/$/, '');
  const links = pageLinks(repo, site); // throws on a site the manifest does not list
  const { code, errors } = await extractCode(repo);
  if (errors.length) throw new Error(`extraction failed:\n${errors.join('\n')}`);
  const pagesDir = path.join(repo.root, repo.manifest.sites[site].pages);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'roundtrip-'));

  let same = 0;
  const differ = [];
  for (const rel of pageFiles(pagesDir).filter((r) => dir === '' || r.startsWith(`${dir}/`))) {
    const original = path.join(from, rel);
    if (!fs.existsSync(original)) {
      differ.push(`${rel}: no original page`);
      continue;
    }
    const a = normalise(fs.readFileSync(original, 'utf-8'));
    const b = normalise(composePage(repo, site, path.join(pagesDir, rel), links, code));
    if (a === b) {
      same++;
      continue;
    }
    const fa = path.join(tmp, 'original.mdx');
    const fb = path.join(tmp, 'composed.mdx');
    fs.writeFileSync(fa, `${a}\n`);
    fs.writeFileSync(fb, `${b}\n`);
    const diff = spawnSync('diff', ['-u', '--label', `original/${rel}`, '--label', `composed/${rel}`, fa, fb], { encoding: 'utf-8' });
    console.log(diff.stdout);
    differ.push(rel);
  }
  fs.rmSync(tmp, { recursive: true, force: true });
  console.log(`${same} pages identical, ${differ.length} differ`);
  for (const d of differ) console.log(`  ${d}`);
  process.exitCode = differ.length > 0 ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
