// In-docs chat widget (AI SDK v7). Mounted once globally from pages/_app.tsx via
// next/dynamic (ssr: false).
//
// Deps:  ai, @ai-sdk/react
//
// v7 notes:
//   - useChat returns { messages, status, sendMessage }; there is no `input` /
//     `handleInputChange` / `handleSubmit`, so the widget owns its input state and
//     calls sendMessage({ text }).
//   - the endpoint is passed via a transport: new DefaultChatTransport({ api }).
//   - messages are UIMessage[]; text lives in `message.parts` (type 'text').
//
// Styling is a deliberate placeholder (floating button + panel). Backlog: open as
// a right-hand sidebar drawer + add an "Ask AI" trigger in the navbar (see the
// scratchpad sequenced backlog).

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

  if (!open) {
    return (
      <button aria-label="Ask AI" onClick={() => setOpen(true)} style={launcherStyle}>
        Ask AI
      </button>
    );
  }

  return (
    <div role="dialog" aria-label="Nym documentation assistant" style={panelStyle}>
      <header style={headerStyle}>
        <span>
          Ask the docs
          {model && <span style={modelStyle}> · {model}</span>}
        </span>
        <button aria-label="Close" onClick={() => setOpen(false)}>
          ×
        </button>
      </header>

      <div style={logStyle}>
        {messages.map((m) => (
          <p key={m.id} style={{ margin: '0.5rem 0' }}>
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
          style={{ flex: 1 }}
        />
        <button type="submit" disabled={status === 'streaming'}>
          Send
        </button>
      </form>

      <div style={footerStyle}>
        <a href="/docs/use-with-ai" style={footerLinkStyle}>
          How to use these docs with AI &rarr;
        </a>
      </div>
    </div>
  );
}

// Placeholder inline styles; replace with the docs design system.
const launcherStyle: React.CSSProperties = { position: 'fixed', bottom: 20, right: 20, zIndex: 50 };
const panelStyle: React.CSSProperties = {
  position: 'fixed',
  bottom: 20,
  right: 20,
  width: 360,
  maxHeight: '70vh',
  display: 'flex',
  flexDirection: 'column',
  zIndex: 50,
  border: '1px solid #2A3235',
  borderRadius: 8,
  background: '#242B2D',
};
const headerStyle: React.CSSProperties = { display: 'flex', justifyContent: 'space-between', padding: '0.5rem 0.75rem' };
const modelStyle: React.CSSProperties = { fontSize: '0.72rem', color: '#9fb0af', fontWeight: 400 };
const logStyle: React.CSSProperties = { flex: 1, overflowY: 'auto', padding: '0.75rem' };
const formStyle: React.CSSProperties = { display: 'flex', gap: 8, padding: '0.5rem 0.75rem' };
const footerStyle: React.CSSProperties = { padding: '0.4rem 0.75rem', borderTop: '1px solid #2A3235' };
const footerLinkStyle: React.CSSProperties = { fontSize: '0.75rem', color: '#85E89D', textDecoration: 'none' };
