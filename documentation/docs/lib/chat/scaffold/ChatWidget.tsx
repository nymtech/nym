// SCAFFOLD - not built, not tested here. Move to `components/ChatWidget.tsx`
// and mount it once, globally, from `pages/_app.tsx` (inside the ThemeProvider,
// beside <AnyComponent />). Lives under lib/chat/scaffold/ (tsconfig-excluded)
// so its AI-SDK import doesn't break `next build` until wired up.
//
// Needs:  pnpm add ai @ai-sdk/react
//
// VERIFY ON INSTALL:
//   - useChat import path: `@ai-sdk/react` (v5) or `ai/react` (v4).
//   - the `api` path includes the /docs basePath, so the request is same-origin
//     with the docs pages and needs no CSP change.
//
// Deliberately minimal: floating button + panel wired to useChat. Styling is a
// placeholder; match it to the Nextra theme when you productionise.

import { useState } from 'react';
import { useChat } from '@ai-sdk/react';

export default function ChatWidget() {
  const [open, setOpen] = useState(false);
  const { messages, input, handleInputChange, handleSubmit, status } = useChat({
    api: '/docs/api/chat', // basePath is /docs, so this is same-origin
  });

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
            <strong>{m.role === 'user' ? 'You' : 'Assistant'}:</strong> {m.content}
          </p>
        ))}
        {status === 'streaming' && <p aria-live="polite">…</p>}
      </div>

      <form onSubmit={handleSubmit} style={formStyle}>
        <input
          value={input}
          onChange={handleInputChange}
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
