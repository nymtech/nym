import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
// @ts-expect-error - plain ESM JS module, no type declarations
import { collectPages } from './pages-source.mjs';

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
