import { describe, it, expect } from 'vitest';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { anchorsOf, anchorLinks, resolveLink } from './anchors.mts';
import { inlinePage } from './inline.mts';

const FIXTURES = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures', 'anchors');
const { tree } = inlinePage(path.join(FIXTURES, 'page.mdx'), { root: FIXTURES });

describe('anchor check', () => {
  it('makes the ids as Nextra does: a slugger per file, explicit anchors, tab labels, no title, no comments', () => {
    expect([...anchorsOf(tree)]).toEqual(['requirements', 'rewards--logic', 'a-step', 'from-the-server', 'from-outside', 'raw']);
  });

  it('adds the ids that the table of a kind citation makes', () => {
    const { tree: cited } = inlinePage(path.join(FIXTURES, 'kind.mdx'), { root: FIXTURES });
    expect([...anchorsOf(cited, { actor: ['actor-L1', 'actor-L2'] })]).toEqual(['actor-L1', 'actor-L2']);
  });

  it('finds the anchor links of the page and its partials, and none in a comment', () => {
    const links = anchorLinks(tree).map((l: { href: string; file: string }) => [l.href, path.basename(l.file)]);
    expect(links).toEqual([['../other.mdx#x', 'partial.mdx'], ['#requirements', 'page.mdx']]);
  });

  it('resolves a link as the browser does from the page URL', () => {
    expect(resolveLink('operators/nodes/maintenance', '#a')).toEqual({ route: 'operators/nodes/maintenance', anchor: 'a' });
    expect(resolveLink('operators/nodes/maintenance', 'nym-node/setup.mdx#b')).toEqual({ route: 'operators/nodes/nym-node/setup', anchor: 'b' });
    expect(resolveLink('operators/troubleshooting/nodes', '../nodes/maintenance.md#c')).toEqual({ route: 'operators/nodes/maintenance', anchor: 'c' });
    expect(resolveLink('x', '/docs/operators/changelog#d')).toEqual({ route: 'operators/changelog', anchor: 'd' });
    expect(resolveLink('', 'operators#e')).toEqual({ route: null, anchor: 'e' });
    expect(resolveLink('', '/operators#e')).toEqual({ route: 'operators', anchor: 'e' });
  });

  it('keeps the extension of an href attribute, which Nextra does not rewrite', () => {
    expect(resolveLink('a/b', 'c.mdx#f', { markdown: false })).toEqual({ route: 'a/c.mdx', anchor: 'f' });
  });
});
