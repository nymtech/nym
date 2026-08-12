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
// Citations reach the client as message metadata, not a response header: v7's
// transport does not surface headers to useChat.

import type { NextApiRequest, NextApiResponse } from 'next';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { anthropic } from '@ai-sdk/anthropic';
import { streamText, convertToModelMessages, safeValidateUIMessages, type UIMessage } from 'ai';
import { buildContext } from '../../lib/chat/context';
import { systemPrompt } from '../../lib/chat/prompt';
import type { DocIndex } from '../../lib/retrieval/types';
// allowJs resolves the plain-ESM embed module; types are inferred from the .mjs.
import { voyageProvider, embedQuery } from '../../lib/retrieval/embed.mjs';

// Loaded once per serverless instance (cold start), then reused.
const index: DocIndex = JSON.parse(readFileSync(path.join(process.cwd(), 'public/docs-index.json'), 'utf-8'));
const embedder = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });

// Sonnet 5 rather than Haiku: the job is synthesising an answer from retrieved
// sections and citing them accurately, and Haiku 4.5 was visibly weaker at it.
// Override per-deployment with CHAT_MODEL; no redeploy of this file needed.
const CHAT_MODEL = process.env.CHAT_MODEL ?? 'claude-sonnet-5';

// Bounds on an incoming request. A docs chat is a short conversation; these cap
// the work a single caller can push through the paid embedding + model calls.
const MAX_MESSAGES = 40;
const MAX_TOTAL_CHARS = 32_000;

// Retrieval floor. `search()` defaults this to 0, and cosine similarity over
// normalised embeddings is effectively never negative, so without a floor every
// question returns a full topK of "sources" no matter how unrelated. That made
// the empty-context branch of systemPrompt() unreachable: the model would say a
// topic was not covered while the widget listed ten sources underneath it.
//
// Starting value, not a measured one. Tune it against a few deliberately
// off-topic questions ("what is the capital of France") and check they come back
// with zero hits while real questions keep theirs.
const MIN_SCORE = Number(process.env.CHAT_MIN_SCORE ?? 0.3);

/** Human-friendly name for the model id, for display in the widget. */
function modelName(id: string): string {
  const map: Record<string, string> = {
    'claude-haiku-4-5': 'Claude Haiku 4.5',
    'claude-sonnet-5': 'Claude Sonnet 5',
    'claude-opus-5': 'Claude Opus 5',
    'claude-opus-4-8': 'Claude Opus 4.8',
  };
  return map[id] ?? id;
}

/** Pull the plain-text query out of a UIMessage's parts (v7 has no .content). */
function textOf(message: UIMessage | undefined): string {
  return (message?.parts ?? [])
    .filter((p): p is { type: 'text'; text: string } => p.type === 'text')
    .map((p) => p.text)
    .join(' ')
    .trim();
}

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method === 'GET') {
    // The widget reads this to show which model is answering.
    res.status(200).json({ model: CHAT_MODEL, name: modelName(CHAT_MODEL) });
    return;
  }
  if (req.method !== 'POST') {
    res.status(405).json({ error: 'Method not allowed' });
    return;
  }

  // Validate the client payload against the exact UIMessage parts shape the SDK
  // expects downstream (convertToModelMessages throws on malformed parts).
  const validation = await safeValidateUIMessages({ messages: req.body?.messages });
  if (!validation.success) {
    res.status(400).json({ error: 'Invalid messages payload' });
    return;
  }
  const messages = validation.data;

  if (messages.length > MAX_MESSAGES) {
    res.status(400).json({ error: 'Too many messages' });
    return;
  }
  const totalChars = messages.reduce((sum, m) => sum + textOf(m).length, 0);
  if (totalChars > MAX_TOTAL_CHARS) {
    res.status(413).json({ error: 'Conversation too large' });
    return;
  }

  const lastUser = [...messages].reverse().find((m) => m.role === 'user');
  const query = textOf(lastUser);
  if (query.length === 0) {
    res.status(400).json({ error: 'No user message to answer' });
    return;
  }

  try {
    // Retrieve over docs only (buildContext defaults sources to ["nym-docs"]).
    const vec = await embedQuery(query, embedder);
    const { context, citations } = buildContext(vec, index, {
      topK: 10,
      minScore: MIN_SCORE,
    });

    const result = streamText({
      model: anthropic(CHAT_MODEL),
      system: systemPrompt(context),
      messages: await convertToModelMessages(messages),
    });

    // Citations ride as message metadata (v7 has no response-header path to
    // useChat). Emitted on `start` so the widget can linkify `[n]` markers as
    // the text streams in, rather than only once the message completes.
    result.pipeUIMessageStreamToResponse(res, {
      messageMetadata: ({ part }) => (part.type === 'start' ? { citations } : undefined),
    });
  } catch (err) {
    // The stream commits the response once it starts; only send a JSON error if
    // we failed before any bytes went out (embedding, retrieval, conversion).
    console.error('chat route failed', err);
    if (!res.headersSent) {
      res.status(500).json({ error: 'Chat request failed' });
    }
  }
}
