import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { MODEL_VIEWS, VARIANT_VIEWS, COLLECTION_VIEWS } from './repo.mts';
import { VIEW_COMPONENTS, COLLECTION_COMPONENTS } from '../retrieval/projections.mjs';

const DOCS = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

describe('views of a threat-model citation', () => {
  it('are the same in the record check, the text projection, and the Snippet component', () => {
    const component = fs.readFileSync(path.join(DOCS, 'components/snippet.tsx'), 'utf-8');
    const block = component.match(/const VIEWS = \{([^]*?)\n\};/)?.[1] ?? '';
    const rendered = [...block.matchAll(/^\s*(\w+):/gm)].map((m) => m[1]);
    expect(rendered).toEqual(MODEL_VIEWS);
    expect(Object.keys(VIEW_COMPONENTS)).toEqual(MODEL_VIEWS);
    const variantViews = JSON.parse(component.match(/const VARIANT_VIEWS = (\[[^\]]*\]);/)?.[1] ?? 'null');
    expect(variantViews).toEqual(VARIANT_VIEWS);
    expect(VARIANT_VIEWS.every((v: string) => MODEL_VIEWS.includes(v))).toBe(true);
    const kinds = Object.fromEntries([...component.matchAll(/^\s*(\w+): \{ view: "(\w+)"/gm)].map((m) => [m[1], [m[2]]]));
    expect(kinds).toEqual(COLLECTION_VIEWS);
    expect(Object.keys(COLLECTION_COMPONENTS)).toEqual(Object.keys(COLLECTION_VIEWS));
  });
});
