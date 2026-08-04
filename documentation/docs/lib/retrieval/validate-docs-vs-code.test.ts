import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { analyseText, SIZE_FACTS } from '../../../scripts/next-scripts/validate-docs-vs-code.mjs';

type Claim = { factId: string; claimedBytes: number; expectedBytes: number; status: 'ok' | 'drift' };

const drift = (cs: Claim[]) => cs.filter((c) => c.status === 'drift');
const ok = (cs: Claim[]) => cs.filter((c) => c.status === 'ok');

describe('docs-vs-code size validation', () => {
  it('flags a packet-size claim that contradicts the source constant', () => {
    // The bug this was built to catch: the page said 2000, the source says 2413.
    const cs = analyseText('Every Sphinx packet is a constant 2000 bytes.') as Claim[];
    expect(drift(cs)).toHaveLength(1);
    expect(drift(cs)[0].factId).toBe('sphinx-packet-size');
    expect(drift(cs)[0].expectedBytes).toBe(2413);
  });

  it('passes the corrected dense sentence without a false positive', () => {
    const cs = analyseText(
      'Each Sphinx packet is a constant 2413 bytes: a 2 KB payload behind a 348-byte routing header.',
    ) as Claim[];
    expect(drift(cs)).toHaveLength(0);
    // packet -> 2413, payload -> 2048, routing header -> 348
    expect(ok(cs)).toHaveLength(3);
  });

  it('accepts an agreeing payload claim (2048) on another page', () => {
    const cs = analyseText('All Sphinx packets have a fixed payload size of 2048 bytes.') as Claim[];
    expect(drift(cs)).toHaveLength(0);
    expect(ok(cs).some((c) => c.factId === 'sphinx-payload-size' && c.claimedBytes === 2048)).toBe(true);
  });

  it('binds each number to the noun it modifies, not the nearest keyword', () => {
    // "payload" sits closer to 2413 than "packet" does; grammatical binding must win.
    const cs = analyseText('Each Sphinx packet is a constant 2413 bytes: a 2 KB payload.') as Claim[];
    const packet = cs.find((c) => c.factId === 'sphinx-packet-size');
    const payload = cs.find((c) => c.factId === 'sphinx-payload-size');
    expect(packet?.claimedBytes).toBe(2413);
    expect(payload?.claimedBytes).toBe(2048);
  });

  it('keeps "payload overhead" (17) distinct from the "payload" size (2048)', () => {
    // Both phrases contain "payload"; longest-noun-first matching must not bind
    // the 17-byte overhead to the 2048-byte payload fact.
    const cs = analyseText('A Sphinx packet has a 2 KB payload and 17 bytes of payload overhead.') as Claim[];
    expect(drift(cs)).toHaveLength(0);
    expect(cs.some((c) => c.factId === 'sphinx-payload-size' && c.claimedBytes === 2048)).toBe(true);
    expect(cs.some((c) => c.factId === 'sphinx-payload-overhead' && c.claimedBytes === 17)).toBe(true);
  });

  it('tolerates adjectives between a number and its noun', () => {
    // "348-byte per-hop routing header": the intervening "per-hop" must not
    // break the bond or shove 348 onto the preceding "payload".
    const cs = analyseText('A Sphinx packet: a 2 KB payload behind a 348-byte per-hop routing header.') as Claim[];
    const header = cs.find((c) => c.factId === 'sphinx-header-size');
    expect(header?.claimedBytes).toBe(348);
    expect(drift(cs)).toHaveLength(0);
  });

  it('ignores common nouns outside a Sphinx sentence (no false positives)', () => {
    // An LP-frame field and a WireGuard MTU both mention "payload"/"byte" but
    // are not the Sphinx payload; the sentence-level context gate drops them.
    const cs = analyseText('The LP frame is [kind: 2 bytes][payload]. WireGuard fills up to a 1500-byte MTU payload.') as Claim[];
    expect(cs).toHaveLength(0);
  });

  it('anchors every fact to a source reference', () => {
    for (const f of SIZE_FACTS as Array<{ source: string; bytes: number }>) {
      expect(typeof f.source).toBe('string');
      expect(f.source.length).toBeGreaterThan(0);
      expect(Number.isInteger(f.bytes)).toBe(true);
    }
  });
});
