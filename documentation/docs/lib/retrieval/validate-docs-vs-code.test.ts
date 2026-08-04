import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { analyseText, SIZE_FACTS, deriveConstants, deriveSizeFacts } from '../../../scripts/next-scripts/validate-docs-vs-code.mjs';

type Claim = { factId: string; dim: string; claimed: number; expected: number; status: 'ok' | 'drift' };

const drift = (cs: Claim[]) => cs.filter((c) => c.status === 'drift');
const ok = (cs: Claim[]) => cs.filter((c) => c.status === 'ok');

describe('docs-vs-code size validation', () => {
  it('flags a packet-size claim that contradicts the source constant', () => {
    // The bug this was built to catch: the page said 2000, the source says 2413.
    const cs = analyseText('Every Sphinx packet is a constant 2000 bytes.') as Claim[];
    expect(drift(cs)).toHaveLength(1);
    expect(drift(cs)[0].factId).toBe('sphinx-packet-size');
    expect(drift(cs)[0].expected).toBe(2413);
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
    expect(ok(cs).some((c) => c.factId === 'sphinx-payload-size' && c.claimed === 2048)).toBe(true);
  });

  it('binds each number to the noun it modifies, not the nearest keyword', () => {
    // "payload" sits closer to 2413 than "packet" does; grammatical binding must win.
    const cs = analyseText('Each Sphinx packet is a constant 2413 bytes: a 2 KB payload.') as Claim[];
    const packet = cs.find((c) => c.factId === 'sphinx-packet-size');
    const payload = cs.find((c) => c.factId === 'sphinx-payload-size');
    expect(packet?.claimed).toBe(2413);
    expect(payload?.claimed).toBe(2048);
  });

  it('keeps "payload overhead" (17) distinct from the "payload" size (2048)', () => {
    // Both phrases contain "payload"; longest-noun-first matching must not bind
    // the 17-byte overhead to the 2048-byte payload fact.
    const cs = analyseText('A Sphinx packet has a 2 KB payload and 17 bytes of payload overhead.') as Claim[];
    expect(drift(cs)).toHaveLength(0);
    expect(cs.some((c) => c.factId === 'sphinx-payload-size' && c.claimed === 2048)).toBe(true);
    expect(cs.some((c) => c.factId === 'sphinx-payload-overhead' && c.claimed === 17)).toBe(true);
  });

  it('tolerates adjectives between a number and its noun', () => {
    // "348-byte per-hop routing header": the intervening "per-hop" must not
    // break the bond or shove 348 onto the preceding "payload".
    const cs = analyseText('A Sphinx packet: a 2 KB payload behind a 348-byte per-hop routing header.') as Claim[];
    const header = cs.find((c) => c.factId === 'sphinx-header-size');
    expect(header?.claimed).toBe(348);
    expect(drift(cs)).toHaveLength(0);
  });

  it('ignores common nouns outside a Sphinx sentence (no false positives)', () => {
    // An LP-frame field and a WireGuard MTU both mention "payload"/"byte" but
    // are not the Sphinx payload; the sentence-level context gate drops them.
    const cs = analyseText('The LP frame is [kind: 2 bytes][payload]. WireGuard fills up to a 1500-byte MTU payload.') as Claim[];
    expect(cs).toHaveLength(0);
  });

  it('anchors every fact to a source reference', () => {
    for (const f of SIZE_FACTS as Array<{ source: string; value: number }>) {
      expect(typeof f.source).toBe('string');
      expect(f.source.length).toBeGreaterThan(0);
      expect(Number.isInteger(f.value)).toBe(true);
    }
  });

  it('validates the IPR MTU (bytes) outside a Sphinx sentence', () => {
    const ok1 = analyseText('The IP packet router caps its IP payload at 1500 bytes.') as Claim[];
    expect(ok1.some((c) => c.factId === 'ipr-max-ip-payload' && c.claimed === 1500 && c.status === 'ok')).toBe(true);
    const bad = analyseText('The IP packet router caps its IP payload at 1400 bytes.') as Claim[];
    expect(drift(bad).some((c) => c.factId === 'ipr-max-ip-payload')).toBe(true);
  });

  it('validates a time claim ("24 hours") against a seconds constant', () => {
    const cs = analyseText('Reply keys expire after 24 hours.') as Claim[];
    const f = cs.find((c) => c.factId === 'reply-key-max-age');
    expect(f?.dim).toBe('time');
    expect(f?.claimed).toBe(24 * 60 * 60);
    expect(f?.status).toBe('ok');
  });

  it('never cross-matches dimensions (a byte number cannot satisfy a time fact)', () => {
    // "reply" context is present, but "512 bytes" is a size, not a time.
    const cs = analyseText('Reply keys are 512 bytes.') as Claim[];
    expect(cs.some((c) => c.factId === 'reply-key-max-age')).toBe(false);
  });
});

describe('oracle derivation from source', () => {
  it('composes the packet size from the in-repo constant expression', () => {
    // REGULAR_PACKET_SIZE = 2*1024 + (HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE)
    const c = deriveConstants() as Record<string, number>;
    expect(c.HEADER_SIZE).toBe(348);
    expect(c.PAYLOAD_OVERHEAD_SIZE).toBe(17);
    expect(c.SPHINX_PACKET_OVERHEAD).toBe(365);
    expect(c.REGULAR_PACKET_SIZE).toBe(2413);
    // Standalone in-repo constants (u16 literal, and a Duration::from_secs expr).
    expect(c.DEFAULT_IPR_TUN_MTU).toBe(1500);
    expect(c.DEFAULT_MAXIMUM_REPLY_KEY_AGE_SECS).toBe(24 * 60 * 60);
  });

  it('derives every oracle fact from source, not hand-typed', () => {
    const byId = Object.fromEntries(
      (deriveSizeFacts() as Array<{ id: string; value: number }>).map((f) => [f.id, f.value]),
    );
    expect(byId['sphinx-packet-size']).toBe(2413);
    expect(byId['sphinx-payload-size']).toBe(2048);
    expect(byId['sphinx-header-size']).toBe(348);
    expect(byId['sphinx-payload-overhead']).toBe(17);
    expect(byId['ipr-max-ip-payload']).toBe(1500);
    expect(byId['reply-key-max-age']).toBe(24 * 60 * 60);
  });

  it('fails loud when the source is missing (never validates against a stale value)', () => {
    expect(() => deriveConstants('/nonexistent-repo-root')).toThrow();
  });
});
