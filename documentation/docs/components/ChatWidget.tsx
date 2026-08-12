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
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

/** Shape of the citation list the route attaches as message metadata. */
interface Citation {
  n: number;
  title: string;
  heading: string;
  url: string;
}

/**
 * Chunk URLs are absolute (`https://nym.com/docs/...`), baked in at index time.
 * Strip the origin so a citation keeps the reader inside whatever deployment
 * they are on: localhost and Vercel previews would otherwise jump to production.
 */
function docsHref(url: string): string {
  return url.replace(/^https?:\/\/[^/]+(?=\/docs\/)/, '');
}

/**
 * Turn the model's inline `[n]` markers into markdown links to the cited
 * section, so react-markdown renders them as anchors.
 *
 * Split on fenced and inline code first: an example containing `arr[0]` must
 * not be rewritten into a link. The negative lookahead leaves real markdown
 * links (`[1](url)`) alone.
 */
function linkifyCitations(text: string, citations: Citation[]): string {
  if (citations.length === 0) return text;
  return text
    .split(/(```[\s\S]*?```|`[^`\n]*`)/g)
    .map((segment, i) =>
      i % 2 === 1
        ? segment
        : segment.replace(/\[(\d+)\](?!\()/g, (whole, n: string) => {
            const c = citations[Number(n) - 1];
            return c ? `[${n}](${docsHref(c.url)})` : whole;
          }),
    )
    .join('');
}

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

  // Push the page content aside while the drawer is open (CSS in pages/styles.css).
  useEffect(() => {
    document.body.classList.toggle('nym-chat-open', open);
    return () => document.body.classList.remove('nym-chat-open');
  }, [open]);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = input.trim();
    if (!text || status === 'streaming') return;
    sendMessage({ text });
    setInput('');
  };

  // Grow the textarea to fit its content, up to a cap. Reset to `auto` first so
  // it can shrink again when text is deleted; scrollHeight only ever grows
  // against a fixed height.
  const resize = (el: HTMLTextAreaElement | null) => {
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.min(el.scrollHeight, INPUT_MAX_HEIGHT)}px`;
  };

  // Enter sends, Shift+Enter breaks the line. Skip while an IME composition is
  // active, or Enter commits the candidate and submits a half-typed message.
  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      submit(e);
    }
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
          // +40px clears the box-shadow/border so nothing bleeds onto the right
          // edge (visible behind the launcher, esp. in dark mode) when closed.
          transform: open ? 'translateX(0)' : 'translateX(calc(100% + 40px))',
          pointerEvents: open ? 'auto' : 'none',
        }}
      >
        <header style={headerStyle}>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            {/* Mix-node hexagon, matching NodeGlyph's shape for mix nodes. */}
            <svg width="14" height="16" viewBox="0 0 14 16" aria-hidden="true">
              <polygon
                points="7,0 13.5,4 13.5,12 7,16 0.5,12 0.5,4"
                fill="var(--chat-accent)"
              />
            </svg>
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
          {messages.map((m) => {
            const text = m.parts
              .filter((p) => p.type === 'text')
              .map((p) => p.text)
              .join('');
            const citations =
              (m.metadata as { citations?: Citation[] } | undefined)?.citations ?? [];
            const isLast = m.id === messages[messages.length - 1]?.id;
            // The caret belongs on the answer being written, not on a finished one.
            const streamingHere = status === 'streaming' && isLast && m.role !== 'user';
            return (
              <div key={m.id} style={m.role === 'user' ? userRowStyle : assistantRowStyle}>
                {m.role === 'user' ? (
                  <p style={userBubbleStyle}>{text}</p>
                ) : (
                  <div style={proseStyle}>
                    <ReactMarkdown remarkPlugins={[remarkGfm]} components={mdComponents}>
                      {linkifyCitations(text, citations)}
                    </ReactMarkdown>
                    {streamingHere && <span className="nym-chat-caret" aria-hidden="true" />}
                    {citations.length > 0 && (
                      <details style={sourcesStyle}>
                        <summary style={{ cursor: 'pointer' }}>
                          Sources ({citations.length})
                        </summary>
                        <ol style={{ margin: '0.4rem 0 0', paddingLeft: '1.2rem' }}>
                          {citations.map((c) => (
                            <li key={c.n} style={{ margin: '0.2rem 0' }}>
                              <a href={docsHref(c.url)} style={linkStyle}>
                                {c.title}
                                {c.heading ? ` - ${c.heading}` : ''}
                              </a>
                            </li>
                          ))}
                        </ol>
                      </details>
                    )}
                  </div>
                )}
              </div>
            );
          })}
          {status === 'submitted' && (
            <div style={assistantRowStyle}>
              <span className="nym-chat-hop" role="status" aria-label="Thinking">
                <span />
                <span />
                <span />
              </span>
            </div>
          )}
        </div>

        <form onSubmit={submit} style={formStyle}>
          <textarea
            ref={resize}
            value={input}
            rows={1}
            onChange={(e) => {
              setInput(e.target.value);
              resize(e.target);
            }}
            onKeyDown={onKeyDown}
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
// Asymmetric turns: the user's words sit in a tinted bubble on the right, the
// answer runs flush-left with no bubble. Answers carry tables, headings and code
// blocks, and a bubble fights that content rather than framing it. The asymmetry
// alone distinguishes the speakers, so no role labels are needed.
const userRowStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  margin: '0 0 0.9rem',
};
const assistantRowStyle: React.CSSProperties = {
  margin: '0 0 1.2rem',
};
const userBubbleStyle: React.CSSProperties = {
  margin: 0,
  padding: '0.5rem 0.75rem',
  maxWidth: '85%',
  background: 'var(--chat-surface)',
  border: '1px solid var(--chat-border)',
  // Square the bottom-right corner so the bubble points at its author.
  borderRadius: '12px 12px 3px 12px',
  fontSize: '0.9rem',
  lineHeight: 1.45,
  whiteSpace: 'pre-wrap',
  overflowWrap: 'anywhere',
};
// Answers are markdown: headings, lists, tables and code all need room. Keeping
// the block scoped here avoids leaking chat styles into the docs theme.
const proseStyle: React.CSSProperties = {
  fontSize: '0.9rem',
  lineHeight: 1.55,
  overflowWrap: 'anywhere',
};
const sourcesStyle: React.CSSProperties = {
  marginTop: '0.7rem',
  padding: '0.4rem 0.6rem',
  borderLeft: '2px solid var(--chat-accent)',
  background: 'var(--chat-surface)',
  borderRadius: '0 6px 6px 0',
  fontSize: '0.8rem',
};
const inlineCodeStyle: React.CSSProperties = {
  background: 'var(--chat-code-bg, rgba(127,127,127,0.16))',
  borderRadius: 4,
  padding: '0.1em 0.35em',
  fontSize: '0.85em',
};
const preStyle: React.CSSProperties = {
  background: 'var(--chat-code-bg, rgba(127,127,127,0.16))',
  borderRadius: 6,
  padding: '0.6rem 0.7rem',
  overflowX: 'auto',
  fontSize: '0.8rem',
};
const tableWrapStyle: React.CSSProperties = { overflowX: 'auto' };

// Headings are demoted: an answer's `##` must not outrank the drawer's own
// title, and the drawer is too narrow for full-size heading type.
const mdComponents = {
  h1: (p: any) => <p style={mdHeadingStyle} {...p} />,
  h2: (p: any) => <p style={mdHeadingStyle} {...p} />,
  h3: (p: any) => <p style={mdHeadingStyle} {...p} />,
  h4: (p: any) => <p style={mdHeadingStyle} {...p} />,
  p: (p: any) => <p style={{ margin: '0 0 0.6rem' }} {...p} />,
  ul: (p: any) => <ul style={{ margin: '0 0 0.6rem', paddingLeft: '1.2rem' }} {...p} />,
  ol: (p: any) => <ol style={{ margin: '0 0 0.6rem', paddingLeft: '1.2rem' }} {...p} />,
  li: (p: any) => <li style={{ margin: '0.15rem 0' }} {...p} />,
  // A citation marker is a link whose entire text is the reference number, which
  // is what linkifyCitations() produces. Ordinary prose links are never bare
  // digits, so this cleanly separates the two without a custom markdown node.
  a: ({ children, ...p }: any) =>
    /^\d+$/.test(String(children)) ? (
      <a className="nym-chat-cite" {...p}>
        {children}
      </a>
    ) : (
      <a style={linkStyle} {...p}>
        {children}
      </a>
    ),
  pre: (p: any) => <pre style={preStyle} {...p} />,
  code: ({ inline, ...p }: any) =>
    inline ? <code style={inlineCodeStyle} {...p} /> : <code {...p} />,
  table: (p: any) => (
    <div style={tableWrapStyle}>
      <table style={{ borderCollapse: 'collapse', width: '100%' }} {...p} />
    </div>
  ),
  th: (p: any) => <th style={mdCellStyle} {...p} />,
  td: (p: any) => <td style={mdCellStyle} {...p} />,
};
const mdHeadingStyle: React.CSSProperties = {
  margin: '0.8rem 0 0.35rem',
  fontWeight: 600,
  fontSize: '0.9rem',
};
const mdCellStyle: React.CSSProperties = {
  border: '1px solid var(--chat-border)',
  padding: '0.3rem 0.45rem',
  textAlign: 'left',
};

// Filled accent pill: reads as an invitation rather than a stray bordered box.
// The ring is a translucent halo of the accent, so it works on either theme
// without a second colour token.
const launcherStyle: React.CSSProperties = {
  position: 'fixed',
  bottom: 20,
  right: 20,
  zIndex: 50,
  alignItems: 'center',
  gap: 7,
  padding: '9px 16px',
  borderRadius: 999,
  border: 'none',
  background: 'var(--chat-accent)',
  color: 'var(--chat-accent-text)',
  fontWeight: 700,
  fontSize: '0.85rem',
  cursor: 'pointer',
  boxShadow: '0 2px 10px var(--chat-shadow), 0 0 0 4px color-mix(in srgb, var(--chat-accent) 22%, transparent)',
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
// Roughly six lines before the textarea stops growing and scrolls internally.
const INPUT_MAX_HEIGHT = 140;

const formStyle: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  padding: '0.75rem 1rem',
  borderTop: '1px solid var(--chat-border)',
  alignItems: 'flex-end', // keep Send on the bottom line as the textarea grows
};
const inputStyle: React.CSSProperties = {
  flex: 1,
  padding: '8px 10px',
  borderRadius: 6,
  border: '1px solid var(--chat-border)',
  background: 'var(--chat-input-bg)',
  color: 'var(--chat-text)',
  font: 'inherit',
  lineHeight: 1.45,
  resize: 'none', // height is driven by content, not a drag handle
  overflowY: 'auto',
  maxHeight: INPUT_MAX_HEIGHT,
};
const sendStyle: React.CSSProperties = { padding: '6px 12px', borderRadius: 6, border: 'none', background: 'var(--chat-accent)', color: 'var(--chat-accent-text)', fontWeight: 700, cursor: 'pointer' };
const footerStyle: React.CSSProperties = { padding: '0.5rem 1rem', borderTop: '1px solid var(--chat-border)' };
const linkStyle: React.CSSProperties = { fontSize: '0.8rem', color: 'var(--chat-accent)', textDecoration: 'none' };
