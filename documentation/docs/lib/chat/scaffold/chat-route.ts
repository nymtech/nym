// SCAFFOLD - not built, not tested here. Move to `pages/api/chat.ts` after
// installing deps. Lives under lib/chat/scaffold/, which tsconfig excludes, so
// its AI-SDK imports don't break `next build` until you wire it up.
//
// Needs:  pnpm add ai @ai-sdk/anthropic
// Env:    ANTHROPIC_API_KEY, VOYAGE_API_KEY, optional CHAT_MODEL
// The tested logic it uses (buildContext, systemPrompt, embedQuery) is real.
//
// VERIFY ON INSTALL against the AI SDK version you pin:
//   - streaming to a pages-router Node response. AI SDK v4 exposes
//     `result.pipeDataStreamToResponse(res)`; v5 renamed several helpers.
//     Nextra 2 is the pages router, so you need the Node-response path, NOT
//     the app-router `toDataStreamResponse()` (Web Response).
//   - the message shape useChat sends (v5 may need convertToModelMessages()).

import type { NextApiRequest, NextApiResponse } from 'next';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { anthropic } from '@ai-sdk/anthropic';
import { streamText } from 'ai';
import { buildContext } from '../context';
import { systemPrompt } from '../prompt';
import type { DocIndex } from '../../retrieval/types';
// @ts-expect-error - plain ESM JS module, no type declarations
import { voyageProvider, embedQuery } from '../../retrieval/embed.mjs';

// Loaded once per serverless instance (cold start), then reused.
const index: DocIndex = JSON.parse(readFileSync(path.join(process.cwd(), 'public/docs-index.json'), 'utf-8'));
const embedder = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });

// Default is Haiku 4.5 for a high-volume public docs widget (cost + latency);
// this is decision D2 in the plan. Set CHAT_MODEL=claude-opus-4-8 (or
// claude-sonnet-5) if answer quality needs it.
const CHAT_MODEL = process.env.CHAT_MODEL ?? 'claude-haiku-4-5';

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method !== 'POST') {
    res.status(405).json({ error: 'Method not allowed' });
    return;
  }

  const messages = req.body?.messages ?? [];
  const lastUser = [...messages].reverse().find((m: { role: string }) => m.role === 'user');
  const query = typeof lastUser?.content === 'string' ? lastUser.content : '';

  // Retrieve over docs only (buildContext defaults sources to ["nym-docs"]).
  const vec = await embedQuery(query, embedder);
  const { context, citations } = buildContext(vec, index, { topK: 6 });

  const result = streamText({
    model: anthropic(CHAT_MODEL),
    system: systemPrompt(context),
    messages,
  });

  // Pages-router streaming. Citations ride along in a header the widget reads.
  result.pipeDataStreamToResponse(res, {
    headers: { 'x-nym-citations': encodeURIComponent(JSON.stringify(citations)) },
  });
}
