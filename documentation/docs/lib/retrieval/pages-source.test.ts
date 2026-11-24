import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
// @ts-expect-error - plain ESM JS module, no type declarations
import { collectPages, absoluteLinks } from './pages-source.mjs';

describe('collectPages', () => {
  it('reads a directory index page once, though _meta.js lists it again', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'pages-'));
    fs.mkdirSync(path.join(dir, 'area'));
    fs.writeFileSync(path.join(dir, '_meta.js'), 'export default { area: "Area" }');
    fs.writeFileSync(path.join(dir, 'area', '_meta.js'), 'export default { index: "Overview", child: "Child" }');
    fs.writeFileSync(path.join(dir, 'area', 'index.mdx'), '# Area\n\nThe area text.\n');
    fs.writeFileSync(path.join(dir, 'area', 'child.mdx'), '# Child\n\nThe child text.\n');
    const titles = collectPages(dir).map((p: { title: string }) => p.title);
    expect(titles).toEqual(['Area', 'Child']);
  });
});

describe('absoluteLinks', () => {
  it('prefixes root-relative links and leaves other links and fences alone', () => {
    const body = [
      'See [limits](/developers/limitations#top) and ![diagram](/images/a.png).',
      'Also [here](#local), [rel](./x), [ext](https://github.com/x) and [cdn](//cdn.x/y).',
      '```md',
      '[in a fence](/kept)',
      '```',
    ].join('\n');
    expect(absoluteLinks(body, 'https://nym.com/docs')).toBe([
      'See [limits](https://nym.com/docs/developers/limitations#top) and ![diagram](https://nym.com/docs/images/a.png).',
      'Also [here](#local), [rel](./x), [ext](https://github.com/x) and [cdn](//cdn.x/y).',
      '```md',
      '[in a fence](/kept)',
      '```',
    ].join('\n'));
  });
});
