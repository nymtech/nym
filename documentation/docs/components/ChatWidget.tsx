// In-docs chat widget (AI SDK v7). Opens as a right-hand sidebar drawer. Mounted
// once globally from pages/_app.tsx via next/dynamic (ssr: false), so it never
// participates in SSR/hydration.
//
// Deps:  ai, @ai-sdk/react
//
// v7 notes:
//   - useChat returns { messages, status, sendMessage }; the widget owns its
//     input state and calls sendMessage({ text }).
//   - the endpoint is passed via a transport: new DefaultChatTransport({ api }).
//   - messages are UIMessage[]; text lives in `message.parts` (type 'text').
//
// Styling is still placeholder-grade (inline styles); align with the docs design
// system next. The drawer stays mounted and slides via transform so the
// conversation persists across open/close.

import { useState, useEffect } from 'react';
import { useChat } from '@ai-sdk/react';
import { DefaultChatTransport } from 'ai';

export default function ChatWidget() {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState('');
  const [model, setModel] = useState('');
  const { messages, status, sendMessage } = useChat({
    transport: new DefaultChatTransport({ api: '/docs/api/chat' }), // basePath is /docs
  });

  // The per-page "Ask AI" button opens the widget via this event.
  useEffect(() => {
    const openWidget = () => setOpen(true);
    window.addEventListener('nym:ask-ai', openWidget);
    return () => window.removeEventListener('nym:ask-ai', openWidget);
  }, []);

  // Which model is answering; the server (GET /api/chat) is the source of truth.
  useEffect(() => {
    fetch('/docs/api/chat')
      .then((r) => (r.ok ? r.json() : null))
      .then((d) => d?.name && setModel(d.name))
      .catch(() => {});
  }, []);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = input.trim();
    if (!text || status === 'streaming') return;
    sendMessage({ text });
    setInput('');
  };

  return (
    <>
      <button
        aria-label="Ask AI"
        onClick={() => setOpen(true)}
        style={{ ...launcherStyle, display: open ? 'none' : 'inline-flex' }}
      >
        Ask AI
      </button>

      <aside
        role="dialog"
        aria-label="Nym documentation assistant"
        aria-hidden={!open}
        style={{
          ...drawerStyle,
          transform: open ? 'translateX(0)' : 'translateX(100%)',
          pointerEvents: open ? 'auto' : 'none',
        }}
      >
        <header style={headerStyle}>
          <span>
            Ask the docs
            {model && <span style={modelStyle}> · {model}</span>}
          </span>
          <button aria-label="Close" onClick={() => setOpen(false)} style={closeStyle}>
            ×
          </button>
        </header>

        <div style={logStyle}>
          {messages.length === 0 && (
            <div style={emptyStyle}>
              <p style={{ margin: '0 0 0.75rem' }}>
                Ask about Nym, the SDKs, or running a node. Answers come from the docs, with citations.
              </p>
              <p style={{ margin: 0 }}>
                Building an agent? Point it at our{' '}
                <a href="/docs/developers/mcp" style={linkStyle}>
                  MCP server
                </a>{' '}
                instead, for docs search plus live network tools.
              </p>
            </div>
          )}
          {messages.map((m) => (
            <p key={m.id} style={{ margin: '0.6rem 0' }}>
              <strong>{m.role === 'user' ? 'You' : 'Assistant'}:</strong>{' '}
              {m.parts.filter((p) => p.type === 'text').map((p) => p.text).join('')}
            </p>
          ))}
          {status === 'streaming' && <p aria-live="polite">…</p>}
        </div>

        <form onSubmit={submit} style={formStyle}>
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask about Nym, the SDKs, running a node…"
            aria-label="Your question"
            style={inputStyle}
          />
          <button type="submit" disabled={status === 'streaming'} style={sendStyle}>
            Send
          </button>
        </form>

        <div style={footerStyle}>
          <a href="/docs/use-with-ai" style={linkStyle}>
            How to use these docs with AI &rarr;
          </a>
        </div>
      </aside>
    </>
  );
}

// Colours reference CSS variables (defined in pages/styles.css) that flip under
// html.dark, so the widget follows the docs light/dark theme. Layout is still
// placeholder-grade; refine to the design system later.
const launcherStyle: React.CSSProperties = {
  position: 'fixed',
  bottom: 20,
  right: 20,
  zIndex: 50,
  padding: '8px 14px',
  borderRadius: 8,
  border: '1px solid var(--chat-border)',
  background: 'var(--chat-bg)',
  color: 'var(--chat-text)',
  cursor: 'pointer',
};
const drawerStyle: React.CSSProperties = {
  position: 'fixed',
  top: 0,
  right: 0,
  height: '100vh',
  width: 'min(420px, 100vw)',
  display: 'flex',
  flexDirection: 'column',
  zIndex: 60,
  background: 'var(--chat-bg)',
  color: 'var(--chat-text)',
  borderLeft: '1px solid var(--chat-border)',
  boxShadow: '-8px 0 24px var(--chat-shadow)',
  transition: 'transform 0.25s ease',
};
const headerStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  padding: '0.75rem 1rem',
  borderBottom: '1px solid var(--chat-border)',
};
const modelStyle: React.CSSProperties = { fontSize: '0.72rem', color: 'var(--chat-text-dim)', fontWeight: 400 };
const closeStyle: React.CSSProperties = { background: 'none', border: 'none', color: 'var(--chat-text-dim)', fontSize: '1.2rem', cursor: 'pointer', lineHeight: 1 };
const logStyle: React.CSSProperties = { flex: 1, overflowY: 'auto', padding: '1rem', fontSize: '0.9rem', lineHeight: 1.5 };
const emptyStyle: React.CSSProperties = { color: 'var(--chat-text-dim)', fontSize: '0.85rem' };
const formStyle: React.CSSProperties = { display: 'flex', gap: 8, padding: '0.75rem 1rem', borderTop: '1px solid var(--chat-border)' };
const inputStyle: React.CSSProperties = { flex: 1, padding: '6px 10px', borderRadius: 6, border: '1px solid var(--chat-border)', background: 'var(--chat-input-bg)', color: 'var(--chat-text)' };
const sendStyle: React.CSSProperties = { padding: '6px 12px', borderRadius: 6, border: 'none', background: 'var(--chat-accent)', color: 'var(--chat-accent-text)', fontWeight: 700, cursor: 'pointer' };
const footerStyle: React.CSSProperties = { padding: '0.5rem 1rem', borderTop: '1px solid var(--chat-border)' };
const linkStyle: React.CSSProperties = { fontSize: '0.8rem', color: 'var(--chat-accent)', textDecoration: 'none' };
