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
  <Banner storageKey="threat-model-2026-08">
    <span>
      New: a threat-model-first guide to{" "}
      <Link href="/network/threat-model" style={{ textDecoration: "underline" }}>
        choosing your network defence
      </Link>
      , plus the{" "}
      <Link href="/developers/smoldvpn" style={{ textDecoration: "underline" }}>
        nym-smoldvpn
      </Link>{" "}
      dVPN package and{" "}
      <Link href="/developers/swizzle" style={{ textDecoration: "underline" }}>
        nym-swizzle
      </Link>{" "}
      sender hygiene.
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

const footer = <Footer>© {new Date().getFullYear()} Nym Technologies SA</Footer>;

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
      editLink={null}
      feedback={{ content: null }}
    >
      {children}
    </Layout>
  );
}
