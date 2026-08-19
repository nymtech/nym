import { describe, it, expect } from 'vitest';
import { validateArgs } from './validate-args';

// Mirrors the real search_docs schema in tools.ts.
const searchSchema = {
  type: 'object',
  properties: {
    query: { type: 'string', description: 'Natural-language search query' },
    topK: { type: 'number', description: 'Max results (default 6)' },
  },
  required: ['query'],
};

// Mirrors the no-argument tools (network_summary, chain_status, etc.).
const emptySchema = { type: 'object', properties: {} };

describe('validateArgs', () => {
  it('accepts args that satisfy the schema', () => {
    expect(validateArgs(searchSchema, { query: 'hi' })).toBeNull();
    expect(validateArgs(searchSchema, { query: 'hi', topK: 3 })).toBeNull();
  });

  it('reports a missing required field', () => {
    const err = validateArgs(searchSchema, {});
    expect(err).not.toBeNull();
    expect(err).toContain('query');
  });

  it('reports a field of the wrong type', () => {
    expect(validateArgs(searchSchema, { query: 42 })).toContain('must be string');
    expect(validateArgs(searchSchema, { query: 'hi', topK: 'lots' })).toContain('must be number');
  });

  it('rejects a non-object argument payload', () => {
    expect(validateArgs(searchSchema, 'nope')).toContain('must be object');
  });

  it('allows extra properties (the tool schemas do not forbid them)', () => {
    expect(validateArgs(searchSchema, { query: 'hi', foo: 1 })).toBeNull();
  });

  it('accepts empty args against a no-argument tool schema', () => {
    expect(validateArgs(emptySchema, {})).toBeNull();
  });
});
