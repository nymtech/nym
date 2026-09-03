import { describe, it, expect, beforeAll } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
// @ts-expect-error - plain ESM JS module, no type declarations
import { parseFrontmatter, stripMdx, inlineMdxPartials } from './mdx.mjs';
// @ts-expect-error - plain ESM JS module
import { loadProjections, loadDocValues } from './projections.mjs';

// A page's source file is not a page's content. Content that renders for a
// reader can be invisible to retrieval by several routes:
//
//   <ActorsReference />      React rendering typed data
//   {RUST_MSRV}              a TypeScript constant
//   <NodePerfMixnet />       another .mdx file
//
// These tests fail at build time, and they are written to catch the *next*
// variant rather than the three known ones: anything a page pulls in that the
// chunker cannot see is a failure here, whatever mechanism it arrives by.

const DOCS_ROOT = path.resolve(__dirname, '../..');
const PAGES = path.join(DOCS_ROOT, 'pages');

function walk(dir: string, acc: string[] = []): string[] {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, acc);
    else if (/\.mdx?$/.test(e.name)) acc.push(full);
  }
  return acc;
}

const pages = walk(PAGES);

// The guard must run the same pipeline the build runs, projections included;
// calling stripMdx bare re-reports every page whose content comes from a
// projected component.
let expand: unknown;
let values: unknown;
beforeAll(async () => {
  expand = await loadProjections();
  values = await loadDocValues();
});

const indexedBody = (page: string) => {
  const { content } = parseFrontmatter(fs.readFileSync(page, 'utf-8'));
  return stripMdx(inlineMdxPartials(content, { filePath: page, root: DOCS_ROOT }), { expand, values });
};

/** Every `import X from '...mdx'` in the docs, with the file it points at. */
function partialImports() {
  const out: { page: string; name: string; spec: string; resolved: string }[] = [];
  for (const page of pages) {
    const src = fs.readFileSync(page, 'utf-8');
    for (const m of src.matchAll(/^import\s+(\w+)\s+from\s+['"]([^'"]+\.mdx)['"]\s*;?\s*$/gm)) {
      const [, name, spec] = m;
      out.push({
        page,
        name,
        spec,
        resolved: spec.startsWith('.')
          ? path.resolve(path.dirname(page), spec)
          : path.resolve(DOCS_ROOT, spec),
      });
    }
  }
  return out;
}

describe('MDX partials reach the index', () => {
  const imports = partialImports();

  it('finds partials at all, so a silent regex break is caught', () => {
    // Without this the suite passes vacuously the moment the import style
    // changes, which is exactly when it needs to fail.
    expect(imports.length).toBeGreaterThan(0);
  });

  it('every imported partial resolves to a file on disk', () => {
    const broken = imports
      .filter((i) => !fs.existsSync(i.resolved))
      .map((i) => `${path.relative(PAGES, i.page)} imports ${i.spec}`);
    expect(broken).toEqual([]);
  });

  it('every partial contributes its text to the page that renders it', () => {
    const lost: string[] = [];
    for (const { page, name, resolved, spec } of imports) {
      // An import is not a render. manual-upgrade.mdx imports two partials and
      // uses neither, so their text is correctly absent. Only a tag that appears
      // in the body puts content on the page.
      const src = fs.readFileSync(page, 'utf-8');
      if (!new RegExp(`<${name}\\s*/>`).test(src)) continue;

      const body = indexedBody(page);

      // Compare against a sentence from the partial rather than its whole text:
      // the partial is chunked and reflowed, so equality is the wrong test.
      const { content: partialBody } = parseFrontmatter(fs.readFileSync(resolved, 'utf-8'));
      const probe = stripMdx(partialBody)
        .split('\n')
        .map((l) => l.trim())
        .find((l) => l.length > 60 && !l.startsWith('|') && !l.startsWith('```'));

      if (probe && !body.includes(probe.slice(0, 60))) {
        lost.push(`${path.relative(PAGES, page)} renders ${spec} but its text is not in the indexed body`);
      }
    }
    expect(lost).toEqual([]);
  });
});

describe('pages are not empty once stripped', () => {
  // A page can carry a dozen characters of indexable text: a heading, and a
  // component holding everything else. A page that renders a screenful and
  // indexes almost nothing is the signature of content the chunker cannot see,
  // whatever the mechanism.
  const MIN_CHARS = 200;

  it('no page collapses to almost nothing after stripping', () => {
    const thin: string[] = [];
    for (const page of pages) {
      const { content } = parseFrontmatter(fs.readFileSync(page, 'utf-8'));
      const raw = content.trim();

      // A short source is usually a short page, but not when the shortness is
      // caused by the content living in a component. pages/index.mdx is 77
      // characters, all of it `<LandingPage />`, so a page only counts as
      // genuinely short if it also renders no component; a length-only exemption
      // would fire on exactly the case this check exists to catch.
      const rendersComponent = /<[A-Z][\w.]*[\s/>]/.test(raw);
      if (raw.length < MIN_CHARS && !rendersComponent) continue;

      const body = indexedBody(page);
      if (body.length < MIN_CHARS) {
        thin.push(`${path.relative(PAGES, page)}: ${raw.length} chars of source -> ${body.length} indexed`);
      }
    }
    // Known and accepted: pages whose entire body is an interactive component
    // with no prose. Listed explicitly so a new one has to be justified rather
    // than quietly joining them.
    const ALLOWED = [
      // Entirely interactive, no prose to lose.
      'developers/playground.mdx',
      // The docs root: a navigation page rendered wholly by <LandingPage />. Its
      // content is a link grid duplicated in better form on /developers, so it
      // is deliberately not projected. Listed rather than exempted by length, so
      // the decision is visible.
      'index.mdx',
    ];
    expect(thin.filter((t) => !ALLOWED.some((a) => t.startsWith(a)))).toEqual([]);
  });
});
