import { describe, it, expect, beforeEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
// @ts-expect-error - plain ESM JS module, no type declarations
import { cutPage, splitPages, componentName, metaJson } from './split.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { buildRecords } from './records.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { openRepo, pageLinks } from './repo.mjs';
// @ts-expect-error - plain ESM JS module, no type declarations
import { composePage, normalise } from './roundtrip.mjs';

const D = 'nym/area/page';

/** A throwaway repository: write `files` under it, return its root. */
function repoWith(files: Record<string, string>) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'snippets-'));
  const all = {
    '.nym-codex/manifest.json': JSON.stringify({ repo: 'nym', github: 'x', sites: { codex: { pages: '.nym-codex/pages', owns: ['area'] } } }),
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
  '.nym-codex/snippets/area/page/intro.mdx': intro,
  '.nym-codex/snippets/area/page/one.mdx': section('one'),
  '.nym-codex/pages/area/page.mdx': page(['intro', 'one']),
};

describe('split', () => {
  it('cuts at ## to #### outside fences, and keeps the intro', () => {
    const { title, sections } = cutPage('# T\n\nlead\n\n## A\n\n```sh\n## not a heading\n```\n\n##### deep stays\n\n#### B\n', { file: 'x' });
    expect(title).toBe('T');
    expect(sections.map((s: { slug: string }) => s.slug)).toEqual(['intro', 'a', 'b']);
    expect(sections[1].heading).toBe('## A [#a]');
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
    expect(writes.get('.nym-codex/snippets/area/page/a-d1.mmd')).toBe('flowchart TD\n  a --> b\n');
    expect(writes.get('.nym-codex/snippets/area/page/a.mdx')).toContain('<Snippet id="nym/area/page/a-d1" />');
    const lead = writes.get('.nym-codex/snippets/area/page/intro.mdx');
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
    const composed = composePage(repo, 'codex', path.join(root, '.nym-codex/pages/area/page.mdx'), pageLinks(repo, 'codex'));
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
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mmd': 'flowchart TD\n' })).toContainEqual(expect.stringContaining(`duplicate id ${D}/one`));
  });

  it('fails on a second heading in a body', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': section('one', '\n\n### Another\n') })).toContainEqual(expect.stringContaining('a second heading'));
  });

  it('fails on a heading anchor that is not the file name', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': '---\naspect: design\n---\n\n## One [#ctors]\n' }))
      .toContainEqual(expect.stringContaining('heading anchor [#ctors] is not the file name [#one]'));
  });

  it('fails on a frontmatter title next to a heading, and on neither', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': '---\ntitle: X\naspect: design\n---\n\n## One [#one]\n' }))
      .toContainEqual(expect.stringContaining('a frontmatter title next to a heading'));
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': '---\naspect: design\n---\n\nno heading\n' }))
      .toContainEqual(expect.stringContaining('no heading and no frontmatter title'));
  });

  it('fails on a first-section title that is not the page title', () => {
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': page(['intro', 'one'], { title: 'Other' }) }))
      .toContainEqual(expect.stringContaining('the first snippet title "Page" is not the page title "Other"'));
  });

  it('fails on an import in a snippet, and on a citation of a prose snippet', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': section('one', "\n\nimport X from './x.mdx'") }))
      .toContainEqual(expect.stringContaining('an import; a snippet imports no file'));
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': section('one', `\n\n<Snippet id="${D}/intro" />`) }))
      .toContainEqual(expect.stringContaining('names a prose snippet'));
  });

  it('fails on a snippet: link with no target', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': section('one', ' See [x](snippet:nym/area/gone).') }))
      .toContainEqual(expect.stringContaining('snippet:nym/area/gone has no snippet'));
  });

  it('fails on a snippet that is not in use, and passes a diagram in use through a citation', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/two.mdx': section('two') })).toContainEqual(`${D}/two: not in use; no page imports it and no imported snippet cites it`);
    expect(errorsOf({
      ...base,
      '.nym-codex/snippets/area/page/one.mdx': section('one', `\n\n<Snippet id="${D}/one-d1" />`),
      '.nym-codex/snippets/area/page/one-d1.mmd': 'flowchart TD\n',
    })).toEqual([]);
  });

  it('fails on a missing or unknown aspect, and on an unknown relation', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': '## One [#one]\n' })).toContainEqual(expect.stringContaining('no aspect'));
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': '---\naspect: why\n---\n\n## One [#one]\n' })).toContainEqual(expect.stringContaining('aspect "why"'));
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs:\n  - { id: ${D}/intro, rel: dependsOn }\n---\n\n## One [#one]\n` }))
      .toContainEqual(expect.stringContaining('relation "dependsOn"'));
  });

  it('normalises refs: a plain id is relates-to, citations are cites, sorted with no duplicate', () => {
    const { prose } = buildRecords(openRepo(repoWith({
      ...base,
      '.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs:\n  - ${D}/intro\n  - { id: ${D}/intro, rel: depends-on }\n---\n\n## One [#one]\n\nSee [i](snippet:${D}/intro).\n`,
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
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': page(['intro', 'one'], { between: '\n\nA paragraph.' }) }))
      .toContainEqual(expect.stringContaining('content with no snippet'));
  });

  it('fails on an import of a snippet that does not exist', () => {
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': page(['intro', 'one', 'gone']) }))
      .toContainEqual(expect.stringContaining(`imports ${D}/gone, which has no snippet file`));
  });

  it('fails on one anchor twice on a page', () => {
    expect(errorsOf({
      ...base,
      '.nym-codex/snippets/area/other/one.mdx': section('one'),
      '.nym-codex/pages/area/page.mdx': `${page(['intro', 'one']).replace('\n\n#', `\nimport Other from '@snippets/nym/area/other/one.mdx'\n\n#`)}<Other />\n`,
    })).toContainEqual(expect.stringContaining('anchor #one is on this page twice'));
  });

  it('allows the Steps wrapper on a how-to page, with only tags inside', () => {
    const steps = `---\ndiataxis: how-to\n---\n${page(['intro', 'one']).replace('<One />', '<Steps>\n<One />\n</Steps>')}`;
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': steps })).toEqual([]);
    const prose = steps.replace('<One />', '<One />\nA paragraph.');
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': prose })).toContainEqual(expect.stringContaining('content with no snippet'));
  });

  it('fails on the Steps wrapper on a page with no runbook type', () => {
    const steps = page(['intro', 'one']).replace('<One />', '<Steps>\n<One />\n</Steps>');
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': steps })).toContainEqual(expect.stringContaining('content with no snippet'));
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
    expect(writes.get('.nym-codex/snippets/area/page/intro.mdx')).toContain('[o](/area/other)');
    expect(report.linksKept).toBe(1);
  });

  it('does not extract a mermaid fence inside an outer example fence', () => {
    const from = repoWith({ 'c/area/page.mdx': '# Page\n\n````md\n```mermaid\nflowchart TD\n```\n````\n' });
    const { writes } = splitPages({ from: path.join(from, 'c'), files: ['area/page.mdx'], repo: 'nym' });
    expect([...writes.keys()].some((k: string) => k.endsWith('.mmd'))).toBe(false);
  });

  it('fails on a code fence above the heading', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': '---\naspect: design\n---\n\n```sh\nx\n```\n\n## One [#one]\n' }))
      .toContainEqual(expect.stringContaining('the heading is not on the first line of the body'));
  });

  it('fails on a setext heading', () => {
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': section('one', '\n\nAnother\n---\n') })).toContainEqual(expect.stringContaining('a second heading'));
  });

  it('fails on a link to a diagram, a cites edge in frontmatter, and refs that are not a list', () => {
    const withDiagram = { ...base, '.nym-codex/snippets/area/page/one-d1.mmd': 'flowchart TD\n' };
    expect(errorsOf({ ...withDiagram, '.nym-codex/snippets/area/page/one.mdx': section('one', ` <Snippet id="${D}/one-d1" /> See [d](snippet:${D}/one-d1).`) }))
      .toContainEqual(expect.stringContaining('names a diagram'));
    expect(errorsOf({ ...withDiagram, '.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs:\n  - { id: ${D}/one-d1, rel: cites }\n---\n\n## One [#one]\n` }))
      .toContainEqual(expect.stringContaining('a cites edge in frontmatter'));
    expect(errorsOf({ ...base, '.nym-codex/snippets/area/page/one.mdx': `---\naspect: design\nrefs: ${D}/intro\n---\n\n## One [#one]\n` }))
      .toContainEqual(expect.stringContaining('refs is not a list'));
  });

  it('fails on a snippet: link to a snippet that no page of the site shows', () => {
    // `two` is in use only through a second site, so the codex pages cannot link to it.
    const files = {
      ...base,
      '.nym-codex/manifest.json': JSON.stringify({ repo: 'nym', github: 'x', sites: { codex: { pages: '.nym-codex/pages' }, docs: { pages: 'docs' } } }),
      '.nym-codex/snippets/area/page/two.mdx': section('two'),
      'docs/other.mdx': `import Two from '@snippets/${D}/two.mdx'\n\n# Other\n\n<Two />\n`,
      '.nym-codex/snippets/area/page/one.mdx': section('one', ` See [t](snippet:${D}/two).`),
    };
    expect(errorsOf(files)).toContainEqual(expect.stringContaining(`snippet:${D}/two is on no page of the site "codex"`));
  });

  it('fails on one import name used twice', () => {
    const twice = `import One from '@snippets/${D}/intro.mdx'\nimport One from '@snippets/${D}/one.mdx'\n\n# Page\n\n<One />\n`;
    expect(errorsOf({ ...base, '.nym-codex/pages/area/page.mdx': twice })).toContainEqual(expect.stringContaining('the import name One is used twice'));
  });

  it('fails on a page that imports a diagram, and warns on an import from another repository', () => {
    const diag = { ...base, '.nym-codex/snippets/area/page/one-d1.mmd': 'flowchart TD\n' };
    const page2 = `${page(['intro', 'one']).replace('\n\n#', `\nimport D1 from '@snippets/${D}/one-d1.mdx'\n\n#`)}<D1 />\n`;
    expect(errorsOf({ ...diag, '.nym-codex/pages/area/page.mdx': page2 })).toContainEqual(`${D}/one-d1: a page imports a diagram; a prose snippet cites it`);
    const other = `${page(['intro', 'one']).replace('\n\n#', `\nimport V from '@snippets/vpn/x/y.mdx'\n\n#`)}<V />\n`;
    const result = buildRecords(openRepo(repoWith({ ...base, '.nym-codex/pages/area/page.mdx': other })));
    expect(result.warnings).toContainEqual(expect.stringContaining('vpn/x/y is from another repository'));
  });

  it('names the page, the import line, and the snippet line of a compose error', () => {
    const root = repoWith({ ...base, '.nym-codex/snippets/area/page/one.mdx': section('one', `\n\n<Snippet id="${D}/gone-d1" />`) });
    const repo = openRepo(root);
    expect(() => composePage(repo, 'codex', path.join(root, '.nym-codex/pages/area/page.mdx'), pageLinks(repo, 'codex')))
      .toThrow(/page\.mdx:2: in '@snippets\/nym\/area\/page\/one\.mdx': .*one\.mdx:9: citation nym\/area\/page\/gone-d1 has no snippet/);
  });
});
