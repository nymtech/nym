import { describe, it, expect } from 'vitest';
import { systemPrompt, SYSTEM_PROMPT_BASE } from './prompt';

describe('systemPrompt', () => {
  it('embeds the context and instructs citing [n] and context-only answering', () => {
    const p = systemPrompt('[1] Page - install\nrun npm i');
    expect(p).toContain(SYSTEM_PROMPT_BASE);
    expect(p).toContain('run npm i');
    expect(p).toContain('[n]');
    expect(p).toContain('ONLY');
  });

  it('requires a marker on every claim, since the source list is built from them', () => {
    // Weaken this and answers that cite in prose show no sources at all.
    const p = systemPrompt('[1] Page - install\nrun npm i');
    expect(p).toContain('Every factual claim must carry at least one [n] marker');
    expect(p).toContain('cite only the ones you used');
  });

  it('tells the model to cite nothing when the context does not answer the question', () => {
    // This is the signal the widget reads to suppress the source list.
    expect(systemPrompt('[1] Page - install\nrun npm i')).toContain('cite nothing');
  });

  it('switches to a no-context guardrail when nothing was retrieved', () => {
    const p = systemPrompt('   ');
    expect(p).toContain('could not');
    expect(p).toContain('Do not answer from general knowledge');
    expect(p).not.toContain('Context:');
  });
});

describe('scope honesty', () => {
  // Retrieval is agreement-biased: "I want total anonymity" retrieves the
  // "Protected:" sections and not the "Unprotected:" ones sitting beside them.
  // The correction has to live in the prompt, because no amount of rewriting the
  // disclaimers moves them closer to a query that presumes the opposite.
  it('states the application-layer limit as a standing fact, not a retrieved one', () => {
    const p = systemPrompt('[1] Page - anything\nbody');
    expect(p).toMatch(/does not make an application private/i);
    expect(p).toMatch(/logins, cookies, API tokens/i);
  });

  it('requires an over-claim to be challenged before the answer', () => {
    const p = systemPrompt('[1] Page - anything\nbody');
    expect(p).toMatch(/assumes more protection than Nym provides/i);
    expect(p).toMatch(/Never let an over-claim stand unchallenged/i);
  });

  it('carries the limit even when nothing was retrieved', () => {
    // The no-context branch is exactly where an over-claim is most likely: the
    // question was vague enough to retrieve nothing, and a bare refusal reads as
    // "no comment" on the premise.
    expect(systemPrompt('   ')).toMatch(/does not make an application private/i);
  });
});

describe('workload fit', () => {
  // Measured failure this guards: asked about a real-time p2p game, the
  // assistant answered "Yes, this is a supported end-to-end setup". It answered
  // the topology question (can two Nym clients talk) and never the workload one
  // (can a mixnet carry real-time updates). The corpus affirms the first loudly
  // and says nothing about the second, so retrieval alone cannot correct it.
  it('names latency and bandwidth as the mechanism, not overhead', () => {
    const p = systemPrompt('[1] Page - anything\nbody');
    expect(p).toMatch(/buys its privacy with latency and bandwidth/i);
    expect(p).toMatch(/not overhead to be tuned away/i);
  });

  it('requires the fit question to be answered before the topology question', () => {
    expect(systemPrompt('[1] P - x\nbody')).toMatch(
      /answer that question before answering any question about\s+topology/i,
    );
  });

  it('recommends dVPN for bulk traffic to a clearnet destination', () => {
    const p = systemPrompt('[1] P - x\nbody');
    expect(p).toMatch(/recommend dVPN mode/i);
    expect(p).toMatch(/nym-smoldvpn/i);
  });

  it('does not offer dVPN for peer-to-peer real-time traffic', () => {
    // dVPN is a tunnel to a destination. Offering it for traffic between two
    // clients sends the developer to a tool that does not solve their problem
    // either, which is a different failure from refusing outright.
    expect(systemPrompt('[1] P - x\nbody')).toMatch(
      /Do not offer dVPN for peer-to-peer or real-time traffic/i,
    );
  });

  it('rules out detuning the mixnet as an alternative to changing mode', () => {
    expect(systemPrompt('[1] P - x\nbody')).toMatch(
      /Never suggest weakening cover traffic or delays/i,
    );
  });
});
