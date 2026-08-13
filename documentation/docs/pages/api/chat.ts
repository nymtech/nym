// Backend for the in-docs chat widget. Retrieval-augmented: embed the question,
// pull the nearest documentation sections, and stream an answer constrained to
// them. The model is told to answer only from those sections and to cite them,
// which is what lets the assistant say a topic is not covered instead of
// inventing an answer.
//
// Citations ride as message metadata rather than a response header, because the
// client transport does not expose response headers to the widget. That is why
// they are attached to the stream's `start` event below rather than simply set
// on the response.
//
// GET doubles as a health check; see configProblems().

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

/**
 * Configuration problems that make this route unable to answer, checked once at
 * cold start rather than discovered per request.
 *
 * Without this a missing key surfaces as a Voyage 401 or a provider throw deep in
 * the stream, which names neither the variable nor where to set it. ANTHROPIC_API_KEY
 * is the easy one to miss: the provider reads it from the environment itself, so it
 * appears nowhere in this file. A vectorless index is the quiet one: it loads and
 * serves happily while every search returns nothing.
 */
function configProblems(): string[] {
  const problems: string[] = [];
  if (!process.env.VOYAGE_API_KEY) {
    problems.push('VOYAGE_API_KEY is not set: queries cannot be embedded, so retrieval always returns nothing.');
  }
  if (!process.env.ANTHROPIC_API_KEY) {
    problems.push('ANTHROPIC_API_KEY is not set: retrieval works but no answer can be generated.');
  }
  if (!index.embedding?.dim) {
    problems.push('public/docs-index.json has no vectors: it was built without VOYAGE_API_KEY. Rebuild the docs with the key set.');
  }
  return problems;
}

const CONFIG_PROBLEMS = configProblems();
if (CONFIG_PROBLEMS.length > 0) {
  // Lands in the Vercel function log on every cold start, so a misconfigured
  // deployment says so rather than waiting to be reported as bad answers.
  console.error(
    `[chat] disabled, ${CONFIG_PROBLEMS.length} configuration problem(s):\n` +
      CONFIG_PROBLEMS.map((p) => `  - ${p}`).join('\n'),
  );
}

// Model tier matters more here than raw speed. The task is synthesising a
// grounded answer from several retrieved sections and attributing each claim to
// the right one, and a model that is merely fast tends to blur the attribution
// or drift outside the supplied context. CHAT_MODEL overrides this per
// deployment, so a change needs no redeploy of this file.
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

/** A message's text is spread across its parts, so join the text ones back up. */
function textOf(message: UIMessage | undefined): string {
  return (message?.parts ?? [])
    .filter((p): p is { type: 'text'; text: string } => p.type === 'text')
    .map((p) => p.text)
    .join(' ')
    .trim();
}

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method === 'GET') {
    // Doubles as the health check: one curl against a deployment says whether it
    // is wired up, instead of having to ask it a question and read the tea leaves.
    res.status(CONFIG_PROBLEMS.length > 0 ? 503 : 200).json({
      model: CHAT_MODEL,
      name: modelName(CHAT_MODEL),
      ok: CONFIG_PROBLEMS.length === 0,
      chunks: index.chunks.length,
      ...(CONFIG_PROBLEMS.length > 0 ? { problems: CONFIG_PROBLEMS } : {}),
    });
    return;
  }
  if (req.method !== 'POST') {
    res.status(405).json({ error: 'Method not allowed' });
    return;
  }
  if (CONFIG_PROBLEMS.length > 0) {
    // Refuse rather than burn a paid embedding call on a request that cannot be
    // answered, and say why in the response instead of only in the logs.
    res.status(503).json({ error: 'Chat is not configured', problems: CONFIG_PROBLEMS });
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

    // Emitted on `start` rather than `finish` so the widget can turn `[n]`
    // markers into links while the text is still streaming, instead of leaving
    // them as bare digits until the answer completes.
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
