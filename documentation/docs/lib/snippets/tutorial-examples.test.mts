import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Each tutorial example in the SDK (sdk/rust/nym-sdk/examples/tutorial_*.rs) holds
// the complete code of a docs tutorial, so cargo checks that code. Its header
// names the snippet with the same code; the two must stay the same (task 9.15).
const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
const EXAMPLES = path.join(REPO, 'sdk/rust/nym-sdk/examples');

const examples = fs.readdirSync(EXAMPLES).filter((f) => /^tutorial_.*\.rs$/.test(f)).sort();

describe('tutorial examples in the SDK', () => {
  it('exist', () => {
    expect(examples.length).toBeGreaterThan(0);
  });

  for (const file of examples) {
    it(`${file} has the same code as its docs snippet`, () => {
      const source = fs.readFileSync(path.join(EXAMPLES, file), 'utf-8');
      const snippet = source.match(/^\/\/! \((documentation\/docs\/\.nym-codex\/snippets\/[^)]+\.mdx)\)\.$/m)?.[1];
      expect(snippet, `${file} names no snippet in its header`).toBeDefined();
      // The code is everything after the //! header.
      const code = source.replace(/^(\/\/!.*\n)+\n/, '').replace(/\n$/, '');
      const blocks = [...fs.readFileSync(path.join(REPO, snippet!), 'utf-8').matchAll(/```rust\n([^]*?)\n```/g)].map((m) => m[1]);
      expect(blocks).toHaveLength(1);
      expect(blocks[0]).toBe(code);
    });
  }
});
