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

  it('switches to a no-context guardrail when nothing was retrieved', () => {
    const p = systemPrompt('   ');
    expect(p).toContain('could not');
    expect(p).toContain('Do not answer from general knowledge');
    expect(p).not.toContain('Context:');
  });
});
