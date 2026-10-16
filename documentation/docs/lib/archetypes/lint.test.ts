import { describe, it, expect } from 'vitest';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
// @ts-expect-error - plain ESM JS module, no type declarations
import { lintPage, indexPlaceholders, loadJson } from './lint.mjs';

const FIXTURES = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures');
const placeholders = indexPlaceholders(loadJson(path.join(FIXTURES, 'placeholders.json')));
const lint = (name: string) => lintPage(path.join(FIXTURES, name), { root: FIXTURES, placeholders });
const messages = (name: string, rule: string) =>
  lint(name).faults.filter((f: { rule: string }) => f.rule === rule).map((f: { message: string; fix?: string }) => f);

describe('runbook lint', () => {
  it('passes a page in the runbook form', () => {
    expect(lint('pass.mdx').faults).toEqual([]);
  });

  it('reports a step heading with a number, the wrong depth, and no anchor', () => {
    const faults = messages('fail.mdx', 'step-heading').map((f: { message: string }) => f.message);
    expect(faults).toContain('step heading is at depth 6, use depth 3');
    expect(faults).toContain('step heading "1. Stop the node" holds a number; Steps numbers it');
    expect(faults).toContain('step heading "1. Stop the node" has no explicit anchor ([#anchor])');
  });

  it('names the imported file and line of a step heading inside a nested import, at each level', () => {
    const faults = lint('fail.mdx').faults;
    const second = faults.find((f: { message: string }) => f.message.includes('2. Start the node'));
    const third = faults.find((f: { message: string }) => f.message.includes('3. Check the node'));
    expect([second.file, second.line]).toEqual([path.join(FIXTURES, 'partial.mdx'), 3]);
    expect([third.file, third.line]).toEqual([path.join(FIXTURES, 'deeper.mdx'), 1]);
  });

  it('reports a dotted step number', () => {
    const faults = messages('no-title.mdx', 'step-heading').map((f: { message: string }) => f.message);
    expect(faults).toContain('step heading "2.1 Configure" holds a number; Steps numbers it');
  });

  it('passes two procedures that each have a depth 2 heading', () => {
    expect(lint('two-procedures.mdx').faults).toEqual([]);
  });

  it('reports a page with no title and a Steps element with no step heading', () => {
    const faults = messages('no-title.mdx', 'required-sections').map((f: { message: string }) => f.message);
    expect(faults).toEqual(['page has no "Title" section (a depth 1 heading)', 'Steps element holds no step heading']);
  });

  it('reports two procedures with no depth 2 heading each', () => {
    expect(messages('fail.mdx', 'steps-per-procedure')).toHaveLength(2);
  });

  it('reports a fence language with its fix, a fence with none, and an unknown meta key', () => {
    const lang = messages('fail.mdx', 'fence-lang');
    expect(lang).toContainEqual(expect.objectContaining({ message: 'code block language "shell"', fix: 'sh' }));
    expect(lang).toContainEqual(expect.objectContaining({ message: 'code block has no language' }));
    expect(messages('fail.mdx', 'fence-meta')).toContainEqual(expect.objectContaining({ message: 'code block meta "title="x"" is not an allowed key' }));
  });

  it('reports an alias with its name, and an unknown token, and skips tags and heredocs', () => {
    const found = messages('fail.mdx', 'placeholder');
    expect(found).toContainEqual(expect.objectContaining({ message: 'placeholder <NODE_ID> is an alias', fix: '<ID>' }));
    expect(found).toContainEqual(expect.objectContaining({ message: 'placeholder <UNKNOWN_TOKEN> is not in placeholders.json' }));
    expect(found).toContainEqual(expect.objectContaining({ message: 'placeholder <PATH/TO> is not in placeholders.json' }));
    expect(found.some((f: { message: string }) => f.message.includes('<div>') || f.message.includes('EOF'))).toBe(false);
  });

  it('lists a TSX component with no children as unchecked, and it is not a fault', () => {
    const result = lint('fail.mdx');
    expect(result.unchecked).toEqual(['RunTabs']);
    expect(result.faults.some((f: { message: string }) => f.message.includes('RunTabs'))).toBe(false);
  });

  it('checks a tutorial for Steps and a Complete code section', () => {
    const faults = messages('tutorial.mdx', 'required-sections').map((f: { message: string }) => f.message);
    expect(faults).toEqual(['page has no Steps element', 'page has no "Complete code" section (a depth 2 heading "Complete code")']);
  });

  it('finds a Complete code heading that has an explicit anchor', () => {
    expect(lint('tutorial-anchored.mdx').faults).toEqual([]);
  });

  it('lists a member-expression TSX element by its import', () => {
    expect(lint('member.mdx').unchecked).toEqual(['Foo.Bar']);
  });

  it('reports a page that does not parse as a warning, with the file', () => {
    const result = lint('broken.mdx');
    expect(result.faults).toEqual([]);
    expect(result.warnings[0].message).toMatch(/^cannot parse: .*broken\.mdx/);
  });

  it('checks no archetype on a page with no diataxis field', () => {
    expect(lint('none.mdx')).toMatchObject({ type: null, faults: [] });
  });

  it('warns once on an unknown diataxis value and checks nothing', () => {
    const result = lint('unknown.mdx');
    expect(result.faults).toEqual([]);
    expect(result.warnings.map((w: { message: string }) => w.message)).toEqual(['unknown diataxis value "guide"']);
  });

  it('rejects a placeholder list with one token in two entries', () => {
    expect(() => indexPlaceholders([{ name: '<A>', description: '' }, { name: '<B>', description: '', aliases: ['<A>'] }]))
      .toThrow('placeholders.json: <A> is in two entries');
  });
});

describe('placeholders.json', () => {
  it('has no token in two entries, and each entry has a name or a flag', () => {
    const list = loadJson(path.join(FIXTURES, '..', 'placeholders.json'));
    expect(() => indexPlaceholders(list)).not.toThrow();
    for (const entry of list) expect(entry.name ?? entry.flag).toBeTruthy();
  });
});
