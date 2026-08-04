// Injects PageActions at the top of the page content via a client-side portal.
// Done this way, not the theme.config `main` wrapper (which caused a hydration
// mismatch) and not fixed positioning (which hid behind the navbar/banner): a
// portal never takes part in SSR/hydration, and prepending into the content
// column keeps the buttons in-flow at the top of the page.

import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useRouter } from 'next/router';
import PageActions from 'components/PageActions';

function contentEl(): HTMLElement | null {
  return (
    (document.querySelector('main') as HTMLElement | null) ||
    (document.querySelector('.nextra-content') as HTMLElement | null) ||
    (document.querySelector('article') as HTMLElement | null)
  );
}

export default function PageActionsMount() {
  const router = useRouter();
  const [host, setHost] = useState<HTMLElement | null>(null);

  useEffect(() => {
    const node = document.createElement('div');
    const attach = () => {
      const target = contentEl();
      if (target && target.firstChild !== node) target.prepend(node);
    };
    attach();
    setHost(node);
    // Re-attach if Nextra re-renders the content and drops or reorders our node
    // (e.g. on a theme toggle). Cheap: attach() is a query plus a guard.
    const obs = new MutationObserver(attach);
    obs.observe(document.body, { childList: true, subtree: true });
    return () => {
      obs.disconnect();
      node.remove();
    };
  }, [router.asPath]);

  return host ? createPortal(<PageActions />, host) : null;
}
