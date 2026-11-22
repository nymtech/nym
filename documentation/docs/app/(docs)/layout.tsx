import type { ReactNode } from "react";
import Link from "next/link";
import { Footer, Layout, Navbar } from "nextra-theme-docs";
import { Banner } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import { Explorer } from "components/explorer-link";
import { Matrix } from "components/matrix-link";

// Docs chrome for the content routes. Mounts the nextra-theme-docs <Layout> with the page
// map, banner, navbar (plus the Explorer/Matrix links) and footer. editLink/feedback are
// disabled to match the old theme.config.tsx. The route group keeps URLs unchanged.

const banner = (
  <Banner storageKey="nginx2root-cve-2026-11">
    <span>
      19 November 2026: gateway operators, apply the{" "}
      <Link href="/operators/changelog#cve-nginx2root-mitigation" style={{ textDecoration: "underline" }}>
        nginx2root CVE mitigation
      </Link>
      .
    </span>
  </Banner>
);

const navbar = (
  <Navbar
    logo={
      <span
        style={{ fontFamily: "var(--font-mono)", fontSize: "1.1rem", fontWeight: 700 }}
      >
        Nym Docs
      </span>
    }
    projectLink="https://github.com/nymtech/nym"
  >
    <Explorer />
    <Matrix />
  </Navbar>
);

const footer = (
  <Footer>
    <span>
      © {new Date().getFullYear()} Nym Technologies SA ·{" "}
      <Link href="/about/how-these-docs-are-checked">How these docs are checked</Link>
    </span>
  </Footer>
);

export default async function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <Layout
      banner={banner}
      navbar={navbar}
      footer={footer}
      pageMap={await getPageMap()}
      docsRepositoryBase="https://github.com/nymtech/nym/tree/develop/documentation/docs"
      darkMode
      nextThemes={{ defaultTheme: "dark" }}
      sidebar={{ defaultMenuCollapseLevel: 1, autoCollapse: true }}
      // PageActions has its own "Copy page" (the generated .md); the theme's would be a second one.
      copyPageButton={false}
      editLink={null}
      feedback={{ content: null }}
    >
      {children}
    </Layout>
  );
}
