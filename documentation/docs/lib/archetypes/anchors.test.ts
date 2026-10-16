import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { anchorsOf, anchorLinks, resolveLink } from './anchors.mjs';

describe('anchor check', () => {
  it('makes the ids as Nextra does: one slugger per page, explicit anchors, no title, no fences', () => {
    const ids = anchorsOf(['# Title', '## Rewards & `logic`', '## Rewards & logic', '### Step [#a-step]', '```sh', '# a comment', '```', '<div id="raw" />'].join('\n'));
    expect([...ids]).toEqual(['rewards--logic', 'rewards--logic-1', 'a-step', 'raw']);
  });

  it('finds anchor links outside code and skips external ones', () => {
    const links = anchorLinks(['[a](#x) `[b](#y)` [c](https://e.org/#z) <a href="../p#w">', '```', '[d](#v)', '```'].join('\n'));
    expect(links.map((l: { href: string }) => l.href)).toEqual(['#x', '../p#w']);
  });

  it('resolves a link as the browser does from the page URL', () => {
    expect(resolveLink('operators/nodes/maintenance', '#a')).toEqual({ route: 'operators/nodes/maintenance', anchor: 'a' });
    expect(resolveLink('operators/nodes/maintenance', 'nym-node/setup.mdx#b')).toEqual({ route: 'operators/nodes/nym-node/setup', anchor: 'b' });
    expect(resolveLink('operators/troubleshooting/nodes', '../nodes/maintenance.md#c')).toEqual({ route: 'operators/nodes/maintenance', anchor: 'c' });
    expect(resolveLink('x', '/docs/operators/changelog#d')).toEqual({ route: 'operators/changelog', anchor: 'd' });
  });
});
