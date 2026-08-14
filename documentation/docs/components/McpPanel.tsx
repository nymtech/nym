// The "Ask AI" button opens this: instructions for pointing a coding agent at
// the docs MCP server, rather than an in-page chat.
//
// The chat widget it replaces answered from the same index, but through a model
// we prompted. Its scope honesty depended on that prompt, which only the chat
// route could see: an agent on MCP got the raw sections and none of the
// scaffolding. Serving one surface well beat serving two, and the one that
// survives is the one whose answers come from the written text.
//
// The widget lives on in git history, on branch max/docs-ai-chat-widget.

import { useState, useEffect, useCallback } from 'react';
import Link from 'next/link';

const MCP_URL = 'https://nym.com/docs/api/mcp';

const JSON_CONFIG = `{
  "mcpServers": {
    "nym-docs": {
      "type": "http",
      "url": "${MCP_URL}"
    }
  }
}`;

const CLI_CONFIG = `claude mcp add --transport http nym-docs ${MCP_URL}`;

/** A code block with a copy button. Copying is the whole interaction here. */
function Snippet({ label, code }: { label: string; code: string }) {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(() => {
    navigator.clipboard?.writeText(code).then(
      () => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1600);
      },
      () => {}, // clipboard denied: the code is on screen and selectable
    );
  }, [code]);

  return (
    <div style={{ marginBottom: '1rem' }}>
      <div style={labelRowStyle}>
        <span style={labelStyle}>{label}</span>
        <button onClick={copy} style={copyButtonStyle} aria-label={`Copy ${label}`}>
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre style={preStyle}>
        <code>{code}</code>
      </pre>
    </div>
  );
}

export default function McpPanel() {
  const [open, setOpen] = useState(false);

  // The per-page "Ask AI" button opens this by dispatching an event, so the
  // button does not need a reference to the panel.
  useEffect(() => {
    const onOpen = () => setOpen(true);
    window.addEventListener('nym:ask-ai', onOpen);
    return () => window.removeEventListener('nym:ask-ai', onOpen);
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  if (!open) return null;

  return (
    <div style={backdropStyle} onClick={() => setOpen(false)} role="presentation">
      <div
        style={panelStyle}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Use these docs with an AI coding agent"
      >
        <div style={headerStyle}>
          <h2 style={titleStyle}>Use these docs with your coding agent</h2>
          <button onClick={() => setOpen(false)} style={closeStyle} aria-label="Close">
            ×
          </button>
        </div>

        <p style={introStyle}>
          These docs run an <strong>MCP server</strong>. Point an agent at it and it can
          search the documentation and the Nym source, and read live network state, as
          tool calls rather than guesswork. It answers from the written docs, with a link
          to the section every answer came from.
        </p>

        <Snippet label="Claude Code" code={CLI_CONFIG} />
        <Snippet label="Cursor, Codex, and other MCP clients" code={JSON_CONFIG} />

        <p style={noteStyle}>
          For Cursor that goes in <code>.cursor/mcp.json</code>; for Claude Code you can
          use the CLI above or the same JSON in <code>.mcp.json</code> at your project
          root. Any client that speaks Streamable HTTP works: supply the URL, not a stdio
          command. Ask the agent to list the server&rsquo;s tools to confirm it connected.
        </p>

        <p style={introStyle}>
          <Link href="/developers/mcp" style={linkStyle} onClick={() => setOpen(false)}>
            Full tool reference
          </Link>
          {' · '}
          <Link href="/use-with-ai" style={linkStyle} onClick={() => setOpen(false)}>
            Other ways to read these docs with AI
          </Link>
        </p>
      </div>
    </div>
  );
}

const backdropStyle: React.CSSProperties = {
  position: 'fixed',
  inset: 0,
  background: 'rgba(0,0,0,0.55)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  zIndex: 60,
  padding: '1rem',
};

const panelStyle: React.CSSProperties = {
  background: 'var(--chat-surface, #14181a)',
  border: '1px solid var(--chat-border, #2a3134)',
  borderRadius: 10,
  maxWidth: 620,
  width: '100%',
  maxHeight: '85dvh',
  overflowY: 'auto',
  padding: '1.25rem 1.4rem 1.4rem',
  boxShadow: '0 18px 50px rgba(0,0,0,0.45)',
};

const headerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  justifyContent: 'space-between',
  gap: '1rem',
  marginBottom: '0.6rem',
};

const titleStyle: React.CSSProperties = { fontSize: '1.05rem', margin: 0, fontWeight: 600 };

const closeStyle: React.CSSProperties = {
  background: 'none',
  border: 'none',
  color: 'inherit',
  fontSize: '1.5rem',
  lineHeight: 1,
  cursor: 'pointer',
  padding: '0 0.2rem',
};

const introStyle: React.CSSProperties = { fontSize: '0.88rem', lineHeight: 1.55, margin: '0 0 1rem' };
const noteStyle: React.CSSProperties = {
  fontSize: '0.8rem',
  lineHeight: 1.55,
  margin: '0 0 1rem',
  opacity: 0.75,
};

const labelRowStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  marginBottom: '0.3rem',
};

const labelStyle: React.CSSProperties = { fontSize: '0.75rem', opacity: 0.7, letterSpacing: '0.02em' };

const copyButtonStyle: React.CSSProperties = {
  background: 'none',
  border: '1px solid var(--chat-border, #2a3134)',
  borderRadius: 5,
  color: 'inherit',
  fontSize: '0.72rem',
  padding: '0.15rem 0.5rem',
  cursor: 'pointer',
};

const preStyle: React.CSSProperties = {
  background: 'var(--chat-bg, #0e1113)',
  border: '1px solid var(--chat-border, #2a3134)',
  borderRadius: 6,
  padding: '0.7rem 0.8rem',
  overflowX: 'auto',
  fontSize: '0.78rem',
  margin: 0,
};

const linkStyle: React.CSSProperties = { color: 'var(--chat-accent, #6ee7b7)', textDecoration: 'none' };
