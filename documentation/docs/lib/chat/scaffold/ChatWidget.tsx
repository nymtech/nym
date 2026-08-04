// SCAFFOLD - chat widget. Move to `components/ChatWidget.tsx` and mount it once,
// globally, from `pages/_app.tsx` (inside the ThemeProvider). Lives under
// lib/chat/scaffold/ (tsconfig-excluded) so its AI-SDK import doesn't break
// `next build` until wired up.
//
// Needs:  pnpm add ai @ai-sdk/react
//
// VERIFIED against @ai-sdk/react@4 + ai@7 type definitions:
//   - useChat returns { messages, status, sendMessage }; there is no `input` /
//     `handleInputChange` / `handleSubmit` any more, so the widget owns its input
//     state and calls sendMessage({ text }).
//   - the endpoint is passed via a transport: new DefaultChatTransport({ api }).
//   - messages are UIMessage[]; text lives in `message.parts` (type 'text'), not
//     a `.content` string.
//
// Deliberately minimal: floating button + panel. Styling is a placeholder; match
// it to the Nextra theme when you productionise.

import { useState } from 'react';
import { useChat } from '@ai-sdk/react';
import { DefaultChatTransport } from 'ai';

export default function ChatWidget() {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState('');
  const { messages, status, sendMessage } = useChat({
    transport: new DefaultChatTransport({ api: '/docs/api/chat' }), // basePath is /docs
  });

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
        <span>Ask the docs</span>
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
const logStyle: React.CSSProperties = { flex: 1, overflowY: 'auto', padding: '0.75rem' };
const formStyle: React.CSSProperties = { display: 'flex', gap: 8, padding: '0.5rem 0.75rem' };
