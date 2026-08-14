#!/usr/bin/env node
//
// Logging MCP client for the docs server. Every call and its full response are
// appended to transcript.md, so the session is auditable afterwards: an API that
// appears in generated code but not in the transcript did not come from here.
//
// The Vercel protection token is read from a file and never logged.
//
// Usage:
//   node mcp.mjs --list
//   node mcp.mjs search_docs '{"query":"how do I send a message","topK":5}'
//   node mcp.mjs search_code '{"query":"MixnetClient connect_new","topK":5}'
//   node mcp.mjs get_section '{"ref":"https://nym.com/docs/..."}'
//
// curl cannot do TLS in this sandbox (no CA bundle under /etc), so this uses
// node's fetch, which carries its own root certificates.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
// Overridable so a run can keep its own transcript rather than appending to
// whatever the last one left behind.
const TRANSCRIPT = process.env.MCP_TRANSCRIPT ?? path.join(HERE, 'transcript.md');
// Repo root: scripts/agent-trial -> documentation -> repo
const TOKEN_FILE = path.join(HERE, '..', '..', '..', '.bypass');

const BASE =
  process.env.MCP_BASE ??
  'https://docs-nextra-git-max-docs-ai-assistant-mcp-nyx-network-staging.vercel.app';
const URL_ = `${BASE.replace(/\/$/, '')}/docs/api/mcp`;

let token = '';
try {
  token = fs.readFileSync(TOKEN_FILE, 'utf8').trim();
} catch {
  console.error(`No token at ${TOKEN_FILE}. The deployment is protected and every call will return an HTML login page.`);
}

const [tool, rawArgs] = process.argv.slice(2);
if (!tool) {
  console.error('usage: node mcp.mjs <tool|--list> [json-args]');
  process.exit(2);
}

const body =
  tool === '--list'
    ? { jsonrpc: '2.0', id: 1, method: 'tools/list' }
    : {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: { name: tool, arguments: rawArgs ? JSON.parse(rawArgs) : {} },
      };

const res = await fetch(URL_, {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    // Both types are required: Streamable HTTP replies as SSE and the server
    // returns 406 without them.
    Accept: 'application/json, text/event-stream',
    ...(token ? { 'x-vercel-protection-bypass': token } : {}),
  },
  body: JSON.stringify(body),
  signal: AbortSignal.timeout(60_000),
});

const raw = await res.text();

// The reply is an SSE frame wrapping the JSON-RPC result.
const dataLine = raw
  .split('\n')
  .filter((l) => l.startsWith('data: '))
  .map((l) => l.slice(6))
  .join('');

let out;
if (!dataLine) {
  out = `NON-MCP RESPONSE (http ${res.status}). First 300 chars:\n${raw.slice(0, 300)}`;
} else {
  const parsed = JSON.parse(dataLine);
  if (parsed.error) {
    out = `JSON-RPC ERROR: ${JSON.stringify(parsed.error)}`;
  } else if (tool === '--list') {
    out = parsed.result.tools.map((t) => `${t.name}\n  ${t.description}`).join('\n\n');
  } else {
    out = (parsed.result?.content ?? []).map((c) => c.text).join('\n');
    if (parsed.result?.isError) out = `TOOL ERROR: ${out}`;
  }
}

// Append to the transcript. Numbering is derived from the file so restarts do
// not reset it. The token is never part of what gets written.
let n = 1;
try {
  n = (fs.readFileSync(TRANSCRIPT, 'utf8').match(/^### Call /gm) ?? []).length + 1;
} catch {
  fs.writeFileSync(
    TRANSCRIPT,
    `# MCP session transcript\n\nEvery call made against ${URL_}.\nThe protection token is stripped and never recorded.\n`,
  );
}

fs.appendFileSync(
  TRANSCRIPT,
  `\n### Call ${n}: \`${tool}\`\n\n` +
    (rawArgs ? `Arguments:\n\n\`\`\`json\n${rawArgs}\n\`\`\`\n\n` : '\n') +
    `Response (${out.length} chars):\n\n\`\`\`\n${out}\n\`\`\`\n`,
);

console.log(out);
