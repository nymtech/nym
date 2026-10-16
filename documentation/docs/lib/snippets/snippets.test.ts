import { describe, it, expect, beforeEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
// @ts-expect-error - plain ESM JS module, no type declarations
import { cutPage, splitPages, componentName, metaJson } from './split.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { buildRecords } from './records.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { openRepo, pageLinks } from './repo.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { composePage, normalise } from './roundtrip.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { extractCode } from './extract.mjs';

const D = 'nym/area/page';

/** A throwaway repository: write `files` under it, return its root. */
function repoWith(files: Record<string, string>) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'snippets-'));
  const all = {
    'documentation/docs/.nym-codex/manifest.json': JSON.stringify({ repo: 'nym', github: 'x', sites: { codex: { pages: 'documentation/docs/.nym-codex/pages', owns: ['area'] } } }),
    ...files,
  };
  for (const [rel, text] of Object.entries(all)) {
    fs.mkdirSync(path.dirname(path.join(root, rel)), { recursive: true });
    fs.writeFileSync(path.join(root, rel), text);
  }
  return root;
}

const intro = '---\ntitle: "Page"\naspect: design\n---\n\nIntro text.\n';
const section = (slug: string, extra = '') => `---\naspect: design\n---\n\n## Section ${slug} [#${slug}]\n\nBody of ${slug}.${extra}\n`;
const page = (names: string[], { title = 'Page', front = '', between = '' } = {}) =>
  `${front}${names.map((n) => `import ${componentName(n)} from '@snippets/${D}/${n}.mdx'`).join('\n')}\n\n# ${title}\n\n${names.map((n) => `<${componentName(n)} />`).join('\n')}${between}\n`;

function errorsOf(files: Record<string, string>) {
  return buildRecords(openRepo(repoWith(files))).errors as string[];
}

const base = {
  'documentation/docs/.nym-codex/snippets/area/page/intro.mdx': intro,
  'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one'),
  'documentation/docs/.nym-codex/pages/area/page.mdx': page(['intro', 'one']),
};

describe('split', () => {
  it('cuts at ## to #### outside fences, and keeps the intro', () => {
    const { title, sections } = cutPage('# T\n\nlead\n\n## A\n\n```sh\n## not a heading\n```\n\n#### B\n', { file: 'x' });
    expect(title).toBe('T');
    expect(sections.map((s: { slug: string }) => s.slug)).toEqual(['intro', 'a', 'b']);
    expect(sections[1].heading).toBe('## A [#a]');
  });

  it('makes a page wait when a snippet would hold a heading that is not a cut', () => {
    expect(() => cutPage('# T\n\n## A\n\n##### deep\n', { file: 'x' })).toThrow('is a heading that split does not cut');
    expect(() => cutPage('# T\n\n<Callout>\n## Inside\n</Callout>\n', { file: 'x' })).toThrow('is a heading that split does not cut');
  });

  it('removes a component import the site gives, and makes other imports, exports, and values wait', () => {
    const policy = { components: new Map([['Callout', 'nextra/components']]) };
    const { sections } = cutPage("import { Callout } from 'nextra/components'\n\n# T\n\n<Callout>\nhi\n</Callout>\n", { file: 'x', policy });
    expect(sections[0].lines.join('\n')).not.toContain('import');
    expect(() => cutPage("import { Heavy } from 'components/heavy'\n\n# T\n\n<Heavy />\n", { file: 'x', policy })).toThrow('which the site does not give');
    expect(() => cutPage("import { V } from 'x'\n\n# T\n\n{V}\n", { file: 'x', policy })).toThrow('as a value');
    expect(() => cutPage("# T\n\nexport const s = 1\n", { file: 'x', policy })).toThrow('is an export');
  });

  it('cuts headings inside Steps on a how-to page and keeps the wrapper in the page', () => {
    const from = repoWith({ 'c/area/page.mdx': '---\ndiataxis: how-to\n---\n\n# Upgrade\n\nWhy.\n\n<Steps>\n\n### Stop [#stop]\n\nx\n\n### Start [#start]\n\ny\n\n</Steps>\n' });
    const policy = { components: new Map([['Steps', 'nextra/components']]) };
    const { writes, waits } = splitPages({ from: path.join(from, 'c'), files: ['area/page.mdx'], repo: 'nym', policy });
    expect(waits).toEqual([]);
    expect(writes.get('documentation/docs/.nym-codex/pages/area/page.mdx')).toContain('<Intro />\n<Steps>\n<Stop />\n<Start />\n</Steps>');
  });

  it('reserves the intro name and numbers repeated headings', () => {
    const { sections } = cutPage('# T\n\n## Intro\n\n## Code\n\n## Code\n', { file: 'x' });
    expect(sections.map((s: { slug: string }) => s.slug)).toEqual(['intro', 'intro-1', 'code', 'code-1']);
  });

  it('moves a mermaid block to a .mmd file, converts links in the run, and keeps others', () => {
    const from = repoWith({
      'c/area/page.mdx': '# Page\n\nSee [other](/area/other#b) and [top](/area/other) and [out](/elsewhere).\n\n## A\n\n```mermaid\nflowchart TD\n  a --> b\n```\n',
      'c/area/other.mdx': '# Other\n\nlead\n\n## B\n\ntext\n',
    });
    const { writes, report } = splitPages({ from: path.join(from, 'c'), files: ['area/page.mdx', 'area/other.mdx'], repo: 'nym' });
    expect(writes.get('documentation/docs/.nym-codex/snippets/area/page/a-d1.mmd')).toBe('flowchart TD\n  a --> b\n');
    expect(writes.get('documentation/docs/.nym-codex/snippets/area/page/a.mdx')).toContain('<Snippet id="nym/area/page/a-d1" />');
    const lead = writes.get('documentation/docs/.nym-codex/snippets/area/page/intro.mdx');
    expect(lead).toContain('[other](snippet:nym/area/other/b)');
    expect(lead).toContain('[top](snippet:nym/area/other/intro)');
    expect(lead).toContain('[out](/elsewhere)');
    expect(report).toMatchObject({ linksConverted: 2, linksKept: 1, diagrams: 1 });
  });

  it('round trip: a split page composes back to the original', () => {
    const original = '# Page\n\nSee [b](#b) and [o](/area/other).\n\n## B\n\ntext\n\n```mermaid\nflowchart TD\n  a --> b\n```\n';
    const from = repoWith({ 'c/area/page.mdx': original });
    const { writes } = splitPages({ from: path.join(from, 'c'), files: ['area/page.mdx'], repo: 'nym' });
    const root = repoWith(Object.fromEntries(writes));
    const repo = openRepo(root);
    const composed = composePage(repo, 'codex', path.join(root, 'documentation/docs/.nym-codex/pages/area/page.mdx'), pageLinks(repo, 'codex'));
    expect(normalise(composed)).toBe(normalise(original));
  });

  it('writes _meta.json from a _meta.js in key order', () => {
    expect(metaJson('export default {\n  "b": "B",\n  a: "A"\n};\n')).toBe('{\n  "b": "B",\n  "a": "A"\n}\n');
  });
});

describe('record builder', () => {
  it('builds records for a valid tree, with no error', () => {
    const { prose, errors } = buildRecords(openRepo(repoWith(base)));
    expect(errors).toEqual([]);
    expect(prose.map((r: { id: string; title: string }) => [r.id, r.title])).toEqual([
      [`${D}/intro`, 'Page'],
      [`${D}/one`, 'Section one'],
    ]);
    expect(prose[1].body).toBe('Body of one.');
  });

  it('fails on a duplicate id (.mdx and .mmd with one stem)', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mmd': 'flowchart TD\n' })).toContainEqual(expect.stringContaining(`duplicate id ${D}/one`));
  });

  it('fails on a second heading in a body', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', '\n\n### Another\n') })).toContainEqual(expect.stringContaining('a second heading'));
  });

  it('fails on a heading anchor that is not the file name', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': '---\naspect: design\n---\n\n## One [#ctors]\n' }))
      .toContainEqual(expect.stringContaining('heading anchor [#ctors] is not the file name [#one]'));
  });

  it('fails on a frontmatter title next to a heading, and on neither', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': '---\ntitle: X\naspect: design\n---\n\n## One [#one]\n' }))
      .toContainEqual(expect.stringContaining('a frontmatter title next to a heading'));
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': '---\naspect: design\n---\n\nno heading\n' }))
      .toContainEqual(expect.stringContaining('no heading and no frontmatter title'));
  });

  it('fails on a first-section title that is not the page title', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': page(['intro', 'one'], { title: 'Other' }) }))
      .toContainEqual(expect.stringContaining('the first snippet title "Page" is not the page title "Other"'));
  });

  it('fails on an import in a snippet, and on a citation of a prose snippet', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', "\n\nimport X from './x.mdx'") }))
      .toContainEqual(expect.stringContaining('an import; a snippet imports no file'));
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', `\n\n<Snippet id="${D}/intro" />`) }))
      .toContainEqual(expect.stringContaining('names a prose snippet'));
  });

  it('fails on a snippet: link with no target', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', ' See [x](snippet:nym/area/gone).') }))
      .toContainEqual(expect.stringContaining('snippet:nym/area/gone has no snippet'));
  });

  it('fails on a snippet that is not in use, and passes a diagram in use through a citation', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/two.mdx': section('two') })).toContainEqual(`${D}/two: not in use; no page imports it and no imported snippet cites it`);
    expect(errorsOf({
      ...base,
      'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', `\n\n<Snippet id="${D}/one-d1" />`),
      'documentation/docs/.nym-codex/snippets/area/page/one-d1.mmd': 'flowchart TD\n',
    })).toEqual([]);
  });

  it('fails on a missing or unknown aspect, and on an unknown relation', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': '## One [#one]\n' })).toContainEqual(expect.stringContaining('no aspect'));
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': '---\naspect: why\n---\n\n## One [#one]\n' })).toContainEqual(expect.stringContaining('aspect "why"'));
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs:\n  - { id: ${D}/intro, rel: dependsOn }\n---\n\n## One [#one]\n` }))
      .toContainEqual(expect.stringContaining('relation "dependsOn"'));
  });

  it('normalises refs: a plain id is relates-to, citations are cites, sorted with no duplicate', () => {
    const { prose } = buildRecords(openRepo(repoWith({
      ...base,
      'documentation/docs/.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs:\n  - ${D}/intro\n  - { id: ${D}/intro, rel: depends-on }\n---\n\n## One [#one]\n\nSee [i](snippet:${D}/intro).\n`,
    })));
    expect(prose[1].refs).toEqual([{ id: `${D}/intro`, rel: 'depends-on' }, { id: `${D}/intro`, rel: 'relates-to' }]);
  });

  it('gives the same records on two runs', () => {
    const root = repoWith(base);
    expect(JSON.stringify(buildRecords(openRepo(root)))).toBe(JSON.stringify(buildRecords(openRepo(root))));
  });
});

describe('page check', () => {
  it('fails on prose in a page', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': page(['intro', 'one'], { between: '\n\nA paragraph.' }) }))
      .toContainEqual(expect.stringContaining('content with no snippet'));
  });

  it('fails on an import of a snippet that does not exist', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': page(['intro', 'one', 'gone']) }))
      .toContainEqual(expect.stringContaining(`imports ${D}/gone, which has no snippet file`));
  });

  it('fails on one anchor twice on a page', () => {
    expect(errorsOf({
      ...base,
      'documentation/docs/.nym-codex/snippets/area/other/one.mdx': section('one'),
      'documentation/docs/.nym-codex/pages/area/page.mdx': `${page(['intro', 'one']).replace('\n\n#', `\nimport Other from '@snippets/nym/area/other/one.mdx'\n\n#`)}<Other />\n`,
    })).toContainEqual(expect.stringContaining('anchor #one is on this page twice'));
  });

  it('allows the Steps wrapper on a how-to page, with only tags inside', () => {
    const steps = `---\ndiataxis: how-to\n---\n${page(['intro', 'one']).replace('<One />', '<Steps>\n<One />\n</Steps>')}`;
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': steps })).toEqual([]);
    const prose = steps.replace('<One />', '<One />\nA paragraph.');
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': prose })).toContainEqual(expect.stringContaining('content with no snippet'));
  });

  it('fails on the Steps wrapper on a page with no runbook type', () => {
    const steps = page(['intro', 'one']).replace('<One />', '<Steps>\n<One />\n</Steps>');
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': steps })).toContainEqual(expect.stringContaining('content with no snippet'));
  });
});

describe('review findings', () => {
  it('slugs a heading as Nextra does: code span content kept, emphasis removed', () => {
    const { sections } = cutPage('# T\n\n## `setup_env` and *x*\n\n## A **bold** _word_\n', { file: 'x' });
    expect(sections.map((s: { slug: string }) => s.slug)).toEqual(['intro', 'setup_env-and-x', 'a-bold-word']);
  });

  it('fails on two sections with one anchor, explicit or not', () => {
    expect(() => cutPage('# T\n\n## A [#x]\n\n## X\n', { file: 'p.mdx' })).not.toThrow();
    expect(cutPage('# T\n\n## A [#x]\n\n## X\n', { file: 'p.mdx' }).sections.map((s: { slug: string }) => s.slug)).toEqual(['intro', 'x', 'x-1']);
    expect(() => cutPage('# T\n\n## X\n\n## A [#x]\n', { file: 'p.mdx' })).toThrow('p.mdx: two sections have the anchor #x');
    expect(() => cutPage('# T\n\n## Intro [#intro]\n', { file: 'p.mdx' })).toThrow('two sections have the anchor #intro');
  });

  it('keeps the route of a link with no anchor to a page that has no intro', () => {
    const from = repoWith({ 'c/area/page.mdx': '# Page\n\nSee [o](/area/other).\n', 'c/area/other.mdx': '# Other\n\n## B\n\ntext\n' });
    const { writes, report } = splitPages({ from: path.join(from, 'c'), files: ['area/page.mdx', 'area/other.mdx'], repo: 'nym' });
    expect(writes.get('documentation/docs/.nym-codex/snippets/area/page/intro.mdx')).toContain('[o](/area/other)');
    expect(report.linksKept).toBe(1);
  });

  it('does not extract a mermaid fence inside an outer example fence', () => {
    const from = repoWith({ 'c/area/page.mdx': '# Page\n\n````md\n```mermaid\nflowchart TD\n```\n````\n' });
    const { writes } = splitPages({ from: path.join(from, 'c'), files: ['area/page.mdx'], repo: 'nym' });
    expect([...writes.keys()].some((k: string) => k.endsWith('.mmd'))).toBe(false);
  });

  it('fails on a code fence above the heading', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': '---\naspect: design\n---\n\n```sh\nx\n```\n\n## One [#one]\n' }))
      .toContainEqual(expect.stringContaining('the heading is not on the first line of the body'));
  });

  it('fails on a setext heading', () => {
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', '\n\nAnother\n---\n') })).toContainEqual(expect.stringContaining('a second heading'));
  });

  it('fails on a link to a diagram, a cites edge in frontmatter, and refs that are not a list', () => {
    const withDiagram = { ...base, 'documentation/docs/.nym-codex/snippets/area/page/one-d1.mmd': 'flowchart TD\n' };
    expect(errorsOf({ ...withDiagram, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', ` <Snippet id="${D}/one-d1" /> See [d](snippet:${D}/one-d1).`) }))
      .toContainEqual(expect.stringContaining('names a diagram'));
    expect(errorsOf({ ...withDiagram, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs:\n  - { id: ${D}/one-d1, rel: cites }\n---\n\n## One [#one]\n` }))
      .toContainEqual(expect.stringContaining('a cites edge in frontmatter'));
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs: ${D}/intro\n---\n\n## One [#one]\n` }))
      .toContainEqual(expect.stringContaining('refs is not a list'));
  });

  it('fails on a snippet: link to a snippet that no page of the site shows', () => {
    // `two` is in use only through a second site, so the codex pages cannot link to it.
    const files = {
      ...base,
      'documentation/docs/.nym-codex/manifest.json': JSON.stringify({ repo: 'nym', github: 'x', sites: { codex: { pages: 'documentation/docs/.nym-codex/pages' }, docs: { pages: 'docs' } } }),
      'documentation/docs/.nym-codex/snippets/area/page/two.mdx': section('two'),
      'docs/other.mdx': `import Two from '@snippets/${D}/two.mdx'\n\n# Other\n\n<Two />\n`,
      'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', ` See [t](snippet:${D}/two).`),
    };
    expect(errorsOf(files)).toContainEqual(expect.stringContaining(`snippet:${D}/two is on no page of the site "codex"`));
  });

  it('fails on one import name used twice', () => {
    const twice = `import One from '@snippets/${D}/intro.mdx'\nimport One from '@snippets/${D}/one.mdx'\n\n# Page\n\n<One />\n`;
    expect(errorsOf({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': twice })).toContainEqual(expect.stringContaining('the import name One is used twice'));
  });

  it('fails on a page that imports a diagram, and warns on an import from another repository', () => {
    const diag = { ...base, 'documentation/docs/.nym-codex/snippets/area/page/one-d1.mmd': 'flowchart TD\n' };
    const page2 = `${page(['intro', 'one']).replace('\n\n#', `\nimport D1 from '@snippets/${D}/one-d1.mdx'\n\n#`)}<D1 />\n`;
    expect(errorsOf({ ...diag, 'documentation/docs/.nym-codex/pages/area/page.mdx': page2 })).toContainEqual(`${D}/one-d1: a page imports a diagram; a prose snippet cites it`);
    const other = `${page(['intro', 'one']).replace('\n\n#', `\nimport V from '@snippets/vpn/x/y.mdx'\n\n#`)}<V />\n`;
    const result = buildRecords(openRepo(repoWith({ ...base, 'documentation/docs/.nym-codex/pages/area/page.mdx': other })));
    expect(result.warnings).toContainEqual(expect.stringContaining('vpn/x/y is from another repository'));
  });

  it('names the page, the import line, and the snippet line of a compose error', () => {
    const root = repoWith({ ...base, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', `\n\n<Snippet id="${D}/gone-d1" />`) });
    const repo = openRepo(root);
    expect(() => composePage(repo, 'codex', path.join(root, 'documentation/docs/.nym-codex/pages/area/page.mdx'), pageLinks(repo, 'codex')))
      .toThrow(/page\.mdx:2: in '@snippets\/nym\/area\/page\/one\.mdx': .*one\.mdx:9: citation nym\/area\/page\/gone-d1 has no snippet/);
  });
});

describe('site components', () => {
  it('fails on a snippet that renders a component the site does not give', () => {
    const files = {
      ...base,
      'documentation/docs/.nym-codex/manifest.json': JSON.stringify({ repo: 'nym', github: 'x', sites: { codex: { pages: 'documentation/docs/.nym-codex/pages', components: { Callout: 'nextra/components' } } } }),
      'documentation/docs/.nym-codex/snippets/area/page/one.mdx': section('one', '\n\n<Callout>ok</Callout>\n\n<Heavy />'),
    };
    expect(errorsOf(files)).toEqual([`documentation/docs/.nym-codex/pages/area/page.mdx:2: ${D}/one renders <Heavy>, which the site "codex" does not give`]);
    const { prose } = buildRecords(openRepo(repoWith(files)));
    expect(prose[1].meta.components).toEqual(['Callout', 'Heavy']);
  });

  it('mdx-components.jsx gives each component that the manifest lists for the docs site', () => {
    const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
    const manifest = JSON.parse(fs.readFileSync(path.join(root, 'documentation/docs/.nym-codex/manifest.json'), 'utf-8'));
    const file = fs.readFileSync(path.join(root, 'documentation/docs/mdx-components.jsx'), 'utf-8');
    const given = file.match(/const snippetComponents = \{([^}]*)\}/)?.[1].split(',').map((n: string) => n.trim()).filter(Boolean);
    expect(given?.sort()).toEqual(Object.keys(manifest.sites.docs.components).sort());
    for (const [name, source] of Object.entries(manifest.sites.docs.components)) {
      expect(file).toMatch(new RegExp(`import \\{[^}]*\\b${name}\\b[^}]*\\} from "${source}"`));
    }
  });
});

describe('code extractor', () => {
  const RUST = 'pub struct Config {\n    pub a: u8,\n}\n\nimpl Config {\n    pub fn new() -> Self { Config { a: 0 } }\n}\n\nimpl Other {\n    pub fn new() -> Self { Other }\n}\n';
  const cite = (symbol: string) => section('one', `\n\n<Snippet id="nym/src/lib.rs#${symbol}" />`);
  const tree = (symbol: string, rust = RUST) => ({ ...base, 'src/lib.rs': rust, 'documentation/docs/.nym-codex/snippets/area/page/one.mdx': cite(symbol) });

  it('extracts a cited item into a code record, and the record builder takes it', async () => {
    const repo = openRepo(repoWith(tree('Config::new')));
    const { code, errors } = await extractCode(repo);
    expect(errors).toEqual([]);
    expect(code.get('nym/src/lib.rs#Config::new')).toMatchObject({ kind: 'code', aspect: 'implementation', path: 'src/lib.rs', lines: [6, 6], lang: 'rust', title: 'Config::new' });
    const built = buildRecords(repo, { code });
    expect(built.errors).toEqual([]);
    expect(built.prose[1].refs).toContainEqual({ id: 'nym/src/lib.rs#Config::new', rel: 'cites' });
    expect(built.code).toHaveLength(1);
  });

  it('fails on a missing symbol and on an ambiguous one, with the snippet file and line', async () => {
    const missing = await extractCode(openRepo(repoWith(tree('Gone'))));
    expect(missing.errors).toEqual(['documentation/docs/.nym-codex/snippets/area/page/one.mdx:9: nym/src/lib.rs#Gone: no item Gone in src/lib.rs']);
    const ambiguous = await extractCode(openRepo(repoWith(tree('new'))));
    expect(ambiguous.errors[0]).toContain('new names 2 items; qualify it: Config::new (line 6), Other::new (line 10)');
  });

  it('fails when a PR renames the cited struct', async () => {
    const { errors } = await extractCode(openRepo(repoWith(tree('Config', RUST.replaceAll('Config', 'Settings')))));
    expect(errors[0]).toContain('no item Config in src/lib.rs');
  });

  it('keeps the body and its hash when lines are added above the item', async () => {
    const a = (await extractCode(openRepo(repoWith(tree('Config'))))).code.get('nym/src/lib.rs#Config');
    const b = (await extractCode(openRepo(repoWith(tree('Config', `// header\nuse x;\n\n${RUST}`))))).code.get('nym/src/lib.rs#Config');
    expect([b.body, b.hash]).toEqual([a.body, a.hash]);
    expect(b.lines).toEqual([4, 6]);
  });

  it('gives byte-identical code.jsonl on two runs, with several files and citations', async () => {
    const root = repoWith({
      ...tree('Config::new'),
      'src/b.rs': 'pub fn b() {}\npub fn a() {}\n',
      'documentation/docs/.nym-codex/snippets/area/page/intro.mdx': `${intro}\n<Snippet id="nym/src/b.rs#b" />\n\n<Snippet id="nym/src/b.rs#a" />\n\n<Snippet id="nym/src/lib.rs#Config" />\n`,
    });
    const run = async () => {
      const repo = openRepo(root);
      const extracted = await extractCode(repo);
      const built = buildRecords(repo, { code: extracted.code, failed: extracted.failed });
      return built.code.map((r: object) => JSON.stringify(r)).join('\n');
    };
    const first = await run();
    expect(first.split('\n').map((l: string) => JSON.parse(l).id)).toEqual(['nym/src/b.rs#a', 'nym/src/b.rs#b', 'nym/src/lib.rs#Config', 'nym/src/lib.rs#Config::new']);
    expect(await run()).toBe(first);
  });

  it('composes a code citation as a fence in its language', async () => {
    const root = repoWith(tree('Config'));
    const repo = openRepo(root);
    const { code } = await extractCode(repo);
    const text = composePage(repo, 'codex', path.join(root, 'documentation/docs/.nym-codex/pages/area/page.mdx'), pageLinks(repo, 'codex'), code);
    expect(text).toContain('```rust\npub struct Config {\n    pub a: u8,\n}\n```');
  });
});

describe('split review cases (docs pages)', () => {
  const policy = { components: new Map([['Callout', 'nextra/components'], ['Steps', 'nextra/components'], ['Tabs', 'nextra/components'], ['Box', 'components/box']]) };
  const waits = (page: string, extra = {}) => () => cutPage(page, { file: 'x', policy: { ...policy, ...extra } });

  it('does not cut inside a lowercase HTML element, a fragment, or a multi-line comment', () => {
    expect(waits('# T\n\n<details>\n<summary>x</summary>\n\n## Inside\n\nbody\n</details>\n')).toThrow('is a heading that split does not cut');
    expect(waits('# T\n\n<>\n## Inside\n</>\n')).toThrow('is a heading that split does not cut');
    expect(cutPage('# T\n\n{/*\n## not a heading\n*/}\n\n## Real\n', { file: 'x', policy }).sections.map((x: { slug: string }) => x.slug)).toEqual(['intro', 'real']);
  });

  it('makes a page wait on a tag it cannot follow, and on a Steps tag with attributes', () => {
    expect(waits("# T\n\n<Tabs items={[\n  'a',\n]}>\nx\n</Tabs>\n")).toThrow('is a tag this tool cannot follow');
    expect(waits('# T\n\n<Callout>text on the tag line\n</Callout>\n')).toThrow('is a tag this tool cannot follow');
    expect(waits('---\ndiataxis: how-to\n---\n\n# T\n\n<Steps className="big">\n### A [#a]\n</Steps>\n')).toThrow('has attributes or indentation');
    expect(waits('---\ndiataxis: how-to\n---\n\n# T\n\n<Steps>\n### A [#a]\n</ Steps>\n')).toThrow('has indentation or a space');
  });

  it('makes a page wait on a quoted or indented heading, and on a page with no title', () => {
    expect(waits('# T\n\n## A\n\n> ## Quoted\n')).toThrow('is a heading that split does not cut');
    expect(waits('# T\n\n- item\n\n  ## Indented\n')).toThrow('is a heading that split does not cut');
    expect(waits('no title here\n')).toThrow('no "# " title line');
  });

  it('reads two imports on one line', () => {
    const { sections } = cutPage("import { Steps } from 'nextra/components';import { Tabs } from 'nextra/components'\n\n# T\n\nbody\n", { file: 'x', policy });
    expect(sections[0].lines.join('')).not.toContain('import');
  });

  it('compares a relative import with the source the site gives', () => {
    const from = repoWith({ 'docs/content/area/page.mdx': "import { Box } from '../../components/box.tsx'\n\n# T\n\n<Box />\n", 'docs/content/area/wrong.mdx': "import { Box } from '../../other/box'\n\n# T\n\n<Box />\n" });
    const content = path.join(from, 'docs/content');
    const run = (file: string) => splitPages({ from: content, files: [file], repo: 'nym', pagesOut: 'docs/content', policy: { ...policy, importRoot: path.join(from, 'docs') } });
    expect(run('area/page.mdx').waits).toEqual([]);
    expect(run('area/wrong.mdx').waits[0].reason).toContain("but the site gives Box from 'components/box'");
  });
});


describe('excerpt comment', () => {
  // @ts-expect-error - plain ESM JS module, no type declarations
  const load = () => import('./excerpt-diff.mjs');
  const rec = (id: string, body: string) => ({ id, path: 'src/lib.rs', lines: [1, 2], lang: 'rust', body, hash: body });

  it('shows a changed excerpt as a diff with its pages, and nothing when no excerpt changes', async () => {
    const { excerptComment, MARKER } = await load();
    const same = excerptComment({ base: [rec('nym/src/lib.rs#A', 'x')], head: [rec('nym/src/lib.rs#A', 'x')], showing: () => [] });
    expect(same).toBe('');
    const text = excerptComment({ base: [rec('nym/src/lib.rs#A', 'old')], head: [rec('nym/src/lib.rs#A', 'new'), rec('nym/src/lib.rs#B', 'b')], showing: () => ['docs: a.mdx'] });
    expect(text.startsWith(MARKER)).toBe(true);
    expect(text).toContain('-old\n+new');
    expect(text).toContain('`src/lib.rs#B` (new citation)');
    expect(text).toContain('Pages: `docs: a.mdx`');
  });
});

describe('page descriptions from a section list (2b.2)', () => {
  // @ts-expect-error - plain ESM JS module, no type declarations
  const load = () => import('./describe.mjs');

  it('writes a plain-text description, and keeps a description a page already has', async () => {
    const { plainSummary, withDescription } = await load();
    expect(plainSummary('`NymNetworkDetails` and the [table](snippet:x), in *one* place')).toBe('NymNetworkDetails and the table, in one place');
    expect(withDescription('# Page\n', 'A summary')).toBe('---\ndescription: "A summary"\n---\n\n# Page\n');
    expect(withDescription('---\ntitle: T\n---\n# Page\n', 'A summary')).toBe('---\ntitle: T\ndescription: "A summary"\n---\n# Page\n');
    expect(withDescription('---\ndescription: old\n---\n# Page\n', 'new')).toBe(null);
  });
});
