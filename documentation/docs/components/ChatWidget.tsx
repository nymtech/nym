// In-docs chat widget: right-hand drawer, mounted once from pages/_app.tsx with
// ssr: false.
//
// The conversation is component state and is never persisted. So the drawer
// hides by sliding via transform instead of unmounting, and every internal link
// here uses next/link, citations included. A full page load remounts this
// component and the conversation is gone.

import { useState, useEffect } from 'react';
import { useChat } from '@ai-sdk/react';
import { DefaultChatTransport } from 'ai';
import Link from 'next/link';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

// The citation list the route attaches as message metadata. Imported as a type
// so the retrieval module never reaches the client bundle.
import type { Citation } from '../lib/chat/context';
import { docsHref, linkifyCitations, citedSources } from '../lib/chat/citations';

export default function ChatWidget() {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState('');
  const [model, setModel] = useState('');
  const { messages, status, sendMessage, setMessages, stop, error, clearError, regenerate } = useChat({
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

  // 'submitted' is the gap between sending and the first token: an embedding
  // call, retrieval, then Claude's first byte. That takes seconds. Guarding only
  // 'streaming' leaves it accepting a second Enter, and useChat has no in-flight
  // guard, so the second press starts another billed request and the two answers
  // interleave. 'error' stays sendable, to allow a retry.
  const busy = status === 'submitted' || status === 'streaming';

  // Closing the drawer deliberately keeps the conversation, so without this there
  // is no way to start a fresh one short of reloading the page. Stop first: an
  // in-flight stream would otherwise keep appending to the cleared list.
  const newChat = () => {
    stop();
    setMessages([]);
    setInput('');
  };

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = input.trim();
    if (!text || busy) return;
    sendMessage({ text });
    setInput('');
  };

  // Grow the textarea to fit its content, up to a cap. Reset to `auto` first:
  // scrollHeight never reports less than the current fixed height, so without the
  // reset the box grows but never shrinks.
  //
  // Do not wrap this in useCallback. It doubles as the ref callback, and the new
  // identity each render is what makes React reattach and re-measure. That is the
  // only thing that shrinks the box after submit, since setInput('') fires no
  // change event. A stable identity breaks it with no error.
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
          {messages.length > 0 && (
            <button onClick={newChat} style={newChatStyle}>
              New chat
            </button>
          )}
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
                <Link href="/developers/mcp" style={linkStyle}>
                  MCP server
                </Link>{' '}
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
            // Only the answer currently being written gets a caret.
            const streamingHere = status === 'streaming' && isLast && m.role !== 'user';
            // Every retrieved section is a legitimate link target, whether or not
            // it earned a place in the source list, so this stays unfiltered.
            const citeHrefs = new Set(citations.map((c) => docsHref(c.url)));
            const cited = citedSources(text, citations);
            return (
              <div key={m.id} style={m.role === 'user' ? userRowStyle : assistantRowStyle}>
                {m.role === 'user' ? (
                  <p style={userBubbleStyle}>{text}</p>
                ) : (
                  <div className="nym-chat-prose" style={proseStyle}>
                    <ReactMarkdown
                      remarkPlugins={[remarkGfm]}
                      components={makeMdComponents(citeHrefs)}
                    >
                      {linkifyCitations(text, citations)}
                    </ReactMarkdown>
                    {streamingHere && <span className="nym-chat-caret" aria-hidden="true" />}
                    {cited.length > 0 && (
                      <details style={sourcesStyle}>
                        <summary style={{ cursor: 'pointer' }}>
                          Sources ({cited.length})
                        </summary>
                        {/* Numbers are written out rather than left to an <ol>
                            marker. The list holds only the cited sections, so an
                            ordered list would renumber them from 1 and stop
                            matching the [n] markers in the answer above. */}
                        <ul style={{ margin: '0.4rem 0 0', padding: 0, listStyle: 'none' }}>
                          {cited.map((c) => (
                            <li key={c.n} style={{ margin: '0.2rem 0' }}>
                              <span style={sourceNumberStyle}>{c.n}</span>
                              <Link href={docsHref(c.url)} style={linkStyle}>
                                {c.title}
                                {c.heading ? ` - ${c.heading}` : ''}
                              </Link>
                            </li>
                          ))}
                        </ul>
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
          {/* Without this a failed request leaves the question sitting there with
              no answer and no explanation. A missing key on a fresh deployment
              looks identical to the model having nothing to say. */}
          {error && (
            <div style={assistantRowStyle} role="alert">
              <p style={errorStyle}>
                {error.message || 'Something went wrong answering that.'}
              </p>
              <button
                onClick={() => {
                  clearError();
                  regenerate();
                }}
                style={retryStyle}
              >
                Try again
              </button>
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
          {/* Swap rather than disable: a long answer is the case where you most
              want out, and a greyed-out button offers no way to take it. */}
          {busy ? (
            <button type="button" onClick={stop} style={sendStyle}>
              Stop
            </button>
          ) : (
            <button type="submit" style={sendStyle}>
              Send
            </button>
          )}
        </form>

        <div style={footerStyle}>
          <Link href="/use-with-ai" style={linkStyle}>
            How to use these docs with AI &rarr;
          </Link>
        </div>
      </aside>
    </>
  );
}

// Colours reference CSS variables (defined in pages/styles.css) that flip under
// html.dark, so the widget follows the docs light/dark theme. Anything needing a
// selector or a keyframe lives in that stylesheet instead, since inline styles
// can express neither.

// Asymmetric turns: the question sits in a tinted bubble on the right, the answer
// runs flush-left with no bubble. Answers contain tables, headings and code
// blocks, which do not fit inside a bubble at this width. The asymmetry is enough
// to tell the two apart, so there are no role labels.
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
  // Bottom-right corner squared, the usual convention for an outgoing message.
  borderRadius: '12px 12px 3px 12px',
  fontSize: '0.9rem',
  lineHeight: 1.45,
  whiteSpace: 'pre-wrap',
  overflowWrap: 'anywhere',
};
// Answers are markdown: headings, lists, tables and code all need room. The
// class pairs with .nym-chat-prose rules in styles.css, which handle what inline
// styles cannot reach (code inside pre); both stay namespaced so nothing leaks
// into the docs theme.
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
// Echoes the inline `.nym-chat-cite` pill, so a number in the source list reads
// as the same object as the marker it matches in the answer.
const sourceNumberStyle: React.CSSProperties = {
  display: 'inline-block',
  minWidth: '1.1rem',
  marginRight: '0.4rem',
  textAlign: 'center',
  color: 'var(--chat-accent)',
  fontVariantNumeric: 'tabular-nums',
};
const preStyle: React.CSSProperties = {
  background: 'var(--chat-surface)',
  borderRadius: 6,
  padding: '0.6rem 0.7rem',
  overflowX: 'auto',
  fontSize: '0.8rem',
};
const tableWrapStyle: React.CSSProperties = { overflowX: 'auto' };

// Headings render as paragraphs. The drawer is too narrow for heading type, and
// an answer's `##` would compete with the drawer's own title.
// Built per message so the link override can check an href against that answer's
// own citations. `node` is destructured off every override below: react-markdown
// passes the hast node down, and spreading it would put an object on a real DOM
// attribute.
const makeMdComponents = (citeHrefs: Set<string>) => ({
  h1: ({ node, ...p }: any) => <p style={mdHeadingStyle} {...p} />,
  h2: ({ node, ...p }: any) => <p style={mdHeadingStyle} {...p} />,
  h3: ({ node, ...p }: any) => <p style={mdHeadingStyle} {...p} />,
  h4: ({ node, ...p }: any) => <p style={mdHeadingStyle} {...p} />,
  p: ({ node, ...p }: any) => <p style={{ margin: '0 0 0.6rem' }} {...p} />,
  ul: ({ node, ...p }: any) => <ul style={{ margin: '0 0 0.6rem', paddingLeft: '1.2rem' }} {...p} />,
  ol: ({ node, ...p }: any) => <ol style={{ margin: '0 0 0.6rem', paddingLeft: '1.2rem' }} {...p} />,
  li: ({ node, ...p }: any) => <li style={{ margin: '0.15rem 0' }} {...p} />,
  a: ({ children, node, href, ...p }: any) => {
    // Check the href, not the link text. The pill tells the reader a link came
    // from the docs index, and text alone does not establish that: the model can
    // emit `[2](https://elsewhere)` itself, linkifyCitations skips links that are
    // already in link form, and it arrives here looking identical to a citation.
    const isCitation = /^\d+$/.test(String(children)) && citeHrefs.has(href);
    if (isCitation) {
      return (
        <Link href={href} className="nym-chat-cite">
          {children}
        </Link>
      );
    }
    // Internal links route client-side; a full load would remount the widget
    // from _app and discard the conversation.
    return href?.startsWith('/') ? (
      <Link href={href} style={linkStyle}>
        {children}
      </Link>
    ) : (
      <a href={href} style={linkStyle} target="_blank" rel="noopener noreferrer" {...p}>
        {children}
      </a>
    );
  },
  pre: ({ node, ...p }: any) => <pre style={preStyle} {...p} />,
  // Deliberately no `code` override. The renderer gives a component no way to
  // tell an inline span from a fenced block, so that distinction is drawn with a
  // descendant selector instead (.nym-chat-prose code in styles.css).
  table: ({ node, ...p }: any) => (
    <div style={tableWrapStyle}>
      <table style={{ borderCollapse: 'collapse', width: '100%' }} {...p} />
    </div>
  ),
  th: ({ node, ...p }: any) => <th style={mdCellStyle} {...p} />,
  td: ({ node, ...p }: any) => <td style={mdCellStyle} {...p} />,
});
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

const errorStyle: React.CSSProperties = {
  margin: '0 0 0.5rem',
  padding: '0.5rem 0.7rem',
  borderLeft: '2px solid var(--chat-accent)',
  background: 'var(--chat-surface)',
  borderRadius: '0 6px 6px 0',
  fontSize: '0.85rem',
  overflowWrap: 'anywhere',
};
const retryStyle: React.CSSProperties = {
  padding: '4px 10px',
  borderRadius: 999,
  border: '1px solid var(--chat-border)',
  background: 'transparent',
  color: 'var(--chat-text)',
  fontSize: '0.78rem',
  cursor: 'pointer',
};

const newChatStyle: React.CSSProperties = {
  marginLeft: 'auto',
  marginRight: 8,
  padding: '3px 9px',
  borderRadius: 999,
  border: '1px solid var(--chat-border)',
  background: 'transparent',
  color: 'var(--chat-text-dim)',
  fontSize: '0.75rem',
  cursor: 'pointer',
};

// The ring is the accent at low opacity, which avoids defining a second colour
// token per theme.
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
  // dvh, not vh: on mobile Safari 100vh is the viewport with the URL bar hidden,
  // so the input row and Send button sit behind it until the user scrolls. The
  // drawer goes full-width below 420px, so this is a live path, not a corner case.
  height: '100dvh',
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
