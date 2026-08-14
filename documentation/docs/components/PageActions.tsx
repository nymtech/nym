// Per-page action row: "Copy page" (grabs the generated Markdown for this page)
// and "Use with AI" (opens the MCP connection panel). Rendered at the top of every page
// via theme.config.tsx `main`. See the AI-ready docs surface in the plan (3.5).
//
// "Copy page" fetches <path>.md, which generate-page-markdown.mjs emits at build.
// Pages without prose (e.g. the component landing) have no .md; the copy just
// no-ops there. "Use with AI" dispatches a window event McpPanel listens for.

import { useRouter } from 'next/router';
import { useState } from 'react';

export default function PageActions() {
  const router = useRouter();
  const [copied, setCopied] = useState(false);

  const path = router.asPath.split(/[#?]/)[0].replace(/\/$/, '');
  const slug = path === '' ? '/index' : path;
  const mdUrl = `${router.basePath}${slug}.md`;

  const copyPage = async () => {
    try {
      const res = await fetch(mdUrl);
      if (!res.ok) return; // no .md for this page
      await navigator.clipboard.writeText(await res.text());
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      /* offline, or clipboard denied */
    }
  };

  const askAI = () => window.dispatchEvent(new CustomEvent('nym:ask-ai'));

  return (
    <div style={rowStyle}>
      <button type="button" onClick={copyPage} style={btnStyle} title="Copy this page as Markdown">
        {copied ? 'Copied' : 'Copy page'}
      </button>
      <button type="button" onClick={askAI} style={btnStyle} title="Connect these docs to your AI coding agent">
        Use with AI
      </button>
    </div>
  );
}

// Portaled in beside the H1 by PageActionsMount (the host div handles float/position).
// Colours use the shared --chat-* vars, so it follows the theme.
const rowStyle: React.CSSProperties = {
  display: 'flex',
  gap: 8,
};
const btnStyle: React.CSSProperties = {
  fontSize: '0.8rem',
  padding: '4px 12px',
  border: '1px solid var(--chat-border)',
  borderRadius: 6,
  background: 'var(--chat-bg)',
  color: 'var(--chat-text)',
  cursor: 'pointer',
};
