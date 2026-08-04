import { describe, it, expect } from 'vitest';
import {
  // @ts-expect-error - plain ESM JS module, no type declarations
  toSnakeCase,
  rustEnumVariants,
  tsUnionMembers,
  diffParity,
  runCheck,
  // @ts-expect-error - plain ESM JS module, no type declarations
} from '../../../scripts/next-scripts/validate-enum-parity.mjs';

describe('enum-parity checker', () => {
  it('converts PascalCase variants to serde snake_case', () => {
    expect(['Connecting', 'Ready', 'ShuttingDown', 'Shutdown', 'Failed'].map(toSnakeCase)).toEqual([
      'connecting',
      'ready',
      'shutting_down',
      'shutdown',
      'failed',
    ]);
  });

  it('parses Rust enum variants and applies the serde rule, ignoring data payloads', () => {
    const rust = '#[serde(tag = "s", rename_all = "snake_case")]\nenum S { Connecting, ShuttingDown, Failed { reason: R } }';
    expect(rustEnumVariants(rust, 'S')).toEqual(['connecting', 'shutting_down', 'failed']);
  });

  it('parses TS union string literals', () => {
    expect(tsUnionMembers("type T = 'connecting' | 'ready' | 'failed';", 'T')).toEqual([
      'connecting',
      'ready',
      'failed',
    ]);
  });

  it('reports no drift for a matched pair', () => {
    const d = diffParity(['connecting', 'shutting_down'], ['connecting', 'shutting_down']);
    expect(d.missingInTs).toHaveLength(0);
    expect(d.extraInTs).toHaveLength(0);
  });

  it('flags both directions of drift (the D1 shape)', () => {
    const d = diffParity(['shutting_down', 'shutdown'], ['disconnecting', 'disconnected']);
    expect(d.missingInTs).toEqual(['shutting_down', 'shutdown']);
    expect(d.extraInTs).toEqual(['disconnecting', 'disconnected']);
  });

  it('parses the live TunnelState enum from source (both sides read cleanly)', () => {
    // Reads wasm/smolmix/src/state.rs + the TS union; asserts the stable Rust side.
    // The TS side currently drifts (D1); this exercises the real parse without
    // pinning the transient mismatch.
    const r = runCheck(CHECK) as { expected: string[]; actual: string[] };
    expect(r.expected).toEqual(['connecting', 'ready', 'shutting_down', 'shutdown', 'failed']);
    expect(Array.isArray(r.actual)).toBe(true);
  });
});

const CHECK = {
  label: 'TunnelState -> TunnelStateName',
  rustFile: 'wasm/smolmix/src/state.rs',
  rustEnum: 'TunnelState',
  tsFile: 'sdk/typescript/packages/mix-tunnel/src/types.ts',
  tsType: 'TunnelStateName',
};
