// In-docs chat widget (AI SDK v7). Opens as a right-hand sidebar drawer. Mounted
// once globally from pages/_app.tsx via next/dynamic (ssr: false), so it never
// participates in SSR/hydration.
//
// Deps:  ai, @ai-sdk/react, react-markdown, remark-gfm
//
// v7 notes:
//   - useChat returns { messages, status, sendMessage }; the widget owns its
//     input state and calls sendMessage({ text }).
//   - the endpoint is passed via a transport: new DefaultChatTransport({ api }).
//   - messages are UIMessage[]; text lives in `message.parts` (type 'text').
//
// The drawer stays mounted and slides via transform, so the conversation
// persists across open/close. It is also why every internal link here uses
// next/link: a full page load would remount this component and lose it.

import { useState, useEffect } from 'react';
import { useChat } from '@ai-sdk/react';
import { DefaultChatTransport } from 'ai';
import Link from 'next/link';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

// The citation list the route attaches as message metadata. Imported as a type
// so the retrieval module never reaches the client bundle.
import type { Citation } from '../lib/chat/context';

/**
 * Chunk URLs are absolute (`https://nym.com/docs/...`), baked in at index time.
 * Strip the origin so a citation keeps the reader inside whatever deployment
 * they are on: localhost and Vercel previews would otherwise jump to production.
 *
 * The `/docs` basePath comes off too, because these are handed to next/link,
 * which re-adds it. Leaving it on yields `/docs/docs/...`.
 */
function docsHref(url: string): string {
  const path = url.replace(/^https?:\/\/[^/]+/, '');
  // The lookahead keeps a docs-root chunk (`/docs`, `/docs#anchor`) from being
  // mangled, and stops `/docsomething` matching.
  return path.replace(/^\/docs(?=[/#?]|$)/, '') || '/';
}

/**
 * Turn the model's inline `[n]` markers into markdown links to the cited
 * section, so react-markdown renders them as anchors.
 *
 * Split on fenced and inline code first, so an example containing `arr[0]` is
 * not rewritten into a link. The negative lookahead leaves real markdown links
 * (`[1](url)`) alone.
 *
 * Known limits, all cosmetic: this only recognises ``` fences and single-backtick
 * spans, so `[1]` inside a 4-space indented block, a ~~~ fence, or a
 * ``double-backtick`` span still gets linked. A fence that is still streaming has
 * no closing ```, so its contents are briefly treated as prose until the closing
 * chunk arrives, and permanently if the answer is truncated before it. Doing this
 * on the parsed AST (a rehype plugin skipping nodes under code/pre) would be
 * correct by construction.
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

  // 'submitted' covers the gap between sending and the first token, which on this
  // route means a Voyage embedding plus retrieval plus Claude's first byte. That
  // window is seconds long, so guarding only 'streaming' leaves it wide open to a
  // second Enter press, and useChat has no in-flight guard of its own: it would
  // start a second billed request and interleave two answers. 'error' stays
  // sendable so a failed question can be retried.
  const busy = status === 'submitted' || status === 'streaming';

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = input.trim();
    if (!text || busy) return;
    sendMessage({ text });
    setInput('');
  };

  // Grow the textarea to fit its content, up to a cap. Reset to `auto` first so
  // it can shrink again when text is deleted; scrollHeight only ever grows
  // against a fixed height.
  //
  // Do not wrap this in useCallback. It is passed as the ref callback, so a new
  // identity each render is what makes React reattach and re-measure, which is
  // what shrinks the box after submit clears the value. A stable identity would
  // silently break that, because setInput('') fires no change event.
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
            // The caret belongs on the answer being written, not on a finished one.
            const streamingHere = status === 'streaming' && isLast && m.role !== 'user';
            const citeHrefs = new Set(citations.map((c) => docsHref(c.url)));
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
                    {citations.length > 0 && (
                      <details style={sourcesStyle}>
                        <summary style={{ cursor: 'pointer' }}>
                          Sources ({citations.length})
                        </summary>
                        <ol style={{ margin: '0.4rem 0 0', paddingLeft: '1.2rem' }}>
                          {citations.map((c) => (
                            <li key={c.n} style={{ margin: '0.2rem 0' }}>
                              <Link href={docsHref(c.url)} style={linkStyle}>
                                {c.title}
                                {c.heading ? ` - ${c.heading}` : ''}
                              </Link>
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
          <button type="submit" disabled={busy} style={sendStyle}>
            Send
          </button>
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
const preStyle: React.CSSProperties = {
  background: 'var(--chat-surface)',
  borderRadius: 6,
  padding: '0.6rem 0.7rem',
  overflowX: 'auto',
  fontSize: '0.8rem',
};
const tableWrapStyle: React.CSSProperties = { overflowX: 'auto' };

// Headings are demoted: an answer's `##` must not outrank the drawer's own
// title, and the drawer is too narrow for full-size heading type.
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
    // The pill asserts "this came from the docs index", so it must be earned by
    // the href, not by the link text. Checking the text alone let a model-authored
    // `[2](https://elsewhere)` wear the same badge: linkifyCitations skips links
    // that are already in link form, so such a link reaches here untouched.
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
  // No `code` override: react-markdown v9 removed the `inline` prop, so a
  // component cannot distinguish an inline span from a fenced block. That
  // distinction is a descendant selector in styles.css (.nym-chat-prose code).
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
