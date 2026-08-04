// Pages-router chat API route (AI SDK v7). Retrieval-augmented: embed the query,
// pull the top doc sections, and stream a Claude answer grounded in them.
//
// Deps:  ai, @ai-sdk/anthropic
// Env:   ANTHROPIC_API_KEY (generation), VOYAGE_API_KEY (query embedding),
//        optional CHAT_MODEL.
//
// v7 notes (validated live against ai@7.0.51 + @ai-sdk/anthropic@4):
//   - pages-router streaming is `result.pipeUIMessageStreamToResponse(res)`
//     (v4's pipeDataStreamToResponse is gone).
//   - useChat sends UIMessage[] (parts-based); the server converts them with
//     `await convertToModelMessages()` (async) before passing to streamText.
//
// NOTE (follow-up): the v4 scaffold rode citations in an `x-nym-citations`
// response header the widget read. v7's transport model does not surface response
// headers to useChat, so citations-as-links need an in-stream data part or message
// metadata instead. For this first cut the model cites `[n]` inline (the system
// prompt instructs it), referencing the numbered context; wiring `[n]` to source
// URLs is a follow-up. `citations` is still computed here for that next step.

import type { NextApiRequest, NextApiResponse } from 'next';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { anthropic } from '@ai-sdk/anthropic';
import { streamText, convertToModelMessages, type UIMessage } from 'ai';
import { buildContext } from '../../lib/chat/context';
import { systemPrompt } from '../../lib/chat/prompt';
import type { DocIndex } from '../../lib/retrieval/types';
// allowJs resolves the plain-ESM embed module; types are inferred from the .mjs.
import { voyageProvider, embedQuery } from '../../lib/retrieval/embed.mjs';

// Loaded once per serverless instance (cold start), then reused.
const index: DocIndex = JSON.parse(readFileSync(path.join(process.cwd(), 'public/docs-index.json'), 'utf-8'));
const embedder = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });

const CHAT_MODEL = process.env.CHAT_MODEL ?? 'claude-haiku-4-5';

/** Pull the plain-text query out of a UIMessage's parts (v7 has no .content). */
function textOf(message: UIMessage | undefined): string {
  return (message?.parts ?? [])
    .filter((p): p is { type: 'text'; text: string } => p.type === 'text')
    .map((p) => p.text)
    .join(' ')
    .trim();
}

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method !== 'POST') {
    res.status(405).json({ error: 'Method not allowed' });
    return;
  }

  const messages: UIMessage[] = req.body?.messages ?? [];
  const lastUser = [...messages].reverse().find((m) => m.role === 'user');
  const query = textOf(lastUser);

  // Retrieve over docs only (buildContext defaults sources to ["nym-docs"]).
  const vec = await embedQuery(query, embedder);
  const { context } = buildContext(vec, index, { topK: 10 });

  const result = streamText({
    model: anthropic(CHAT_MODEL),
    system: systemPrompt(context),
    messages: await convertToModelMessages(messages),
  });

  result.pipeUIMessageStreamToResponse(res);
}
