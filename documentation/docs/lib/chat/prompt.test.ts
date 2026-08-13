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
