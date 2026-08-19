import { describe, it, expect } from 'vitest';
import { validateSetupMixTunnelOpts } from './validate-config';

describe('validateSetupMixTunnelOpts', () => {
  it('accepts a well-formed config with no errors or warnings', () => {
    const r = validateSetupMixTunnelOpts({ clientId: 'abc', forceTls: true, connectTimeoutMs: 15000 });
    expect(r.valid).toBe(true);
    expect(r.errors).toEqual([]);
    expect(r.warnings).toEqual([]);
  });

  it('rejects a non-object config', () => {
    for (const bad of [null, 42, 'x', ['clientId']]) {
      const r = validateSetupMixTunnelOpts(bad);
      expect(r.valid).toBe(false);
      expect(r.errors[0]).toContain('must be an object');
    }
  });

  it('flags a field with the wrong type as an error, naming the type it got', () => {
    const r = validateSetupMixTunnelOpts({ forceTls: 'yes', openReplySurbs: '10' });
    expect(r.valid).toBe(false);
    expect(r.errors).toContain('Option "forceTls" should be boolean, got string.');
    expect(r.errors).toContain('Option "openReplySurbs" should be number, got string.');
  });

  it('warns (never errors) on an unknown key and suggests the nearest real field', () => {
    const r = validateSetupMixTunnelOpts({ clientID: 'abc' }); // wrong casing
    expect(r.valid).toBe(true); // unknown keys must not fail: the list is a snapshot
    expect(r.warnings.some((w) => w.includes('clientID') && w.includes('clientId'))).toBe(true);
  });

  it('warns without a suggestion when an unknown key is nowhere near a real field', () => {
    const r = validateSetupMixTunnelOpts({ totallyMadeUpOption: 1 });
    expect(r.valid).toBe(true);
    expect(r.warnings.some((w) => w.includes('totallyMadeUpOption'))).toBe(true);
    expect(r.warnings.some((w) => w.includes('Did you mean'))).toBe(false);
  });

  it('notes the anonymity tradeoff when cover or Poisson traffic is disabled', () => {
    const r = validateSetupMixTunnelOpts({ disableCoverTraffic: true, disablePoissonTraffic: true });
    expect(r.valid).toBe(true); // valid config, just a privacy caveat
    expect(r.warnings.some((w) => w.includes('cover traffic'))).toBe(true);
    expect(r.warnings.some((w) => w.includes('Poisson'))).toBe(true);
  });

  it('does not warn about privacy when the flags are their safe (false) default', () => {
    const r = validateSetupMixTunnelOpts({ disableCoverTraffic: false });
    expect(r.warnings).toEqual([]);
  });
});
