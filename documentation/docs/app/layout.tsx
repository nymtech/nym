import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import { Footer, Layout, Navbar } from "nextra-theme-docs";
import { Banner, Head } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import { Explorer } from "components/explorer-link";
import { Matrix } from "components/matrix-link";
import { Providers } from "./providers";
import "nextra-theme-docs/style.css";
import "./styles.css";
import "./threat-model-viz.css";

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://nym.com";
const ogImage = `${siteUrl}/images/Nym_meta_Image.png`;

// Static, site-wide SEO. The per-page title/description, canonical URL and the
// JSON-LD graph are built per route in app/[[...mdxPath]]/page.jsx, which has the
// path and frontmatter.
export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: "Nym Docs: Privacy Network Documentation",
    template: "%s | Nym Docs",
  },
  description:
    "Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials.",
  authors: [{ name: "Nym" }],
  icons: { icon: [{ url: `${siteUrl}/favicon.svg`, type: "image/svg+xml" }] },
  openGraph: {
    type: "article",
    siteName: "Nym docs",
    images: [{ url: ogImage, width: 1200, height: 630 }],
  },
  twitter: {
    card: "summary_large_image",
    site: "@nymproject",
    images: [ogImage],
  },
  appleWebApp: { title: "Nym docs" },
};

const banner = (
  <Banner storageKey="threat-model-2026-08" dismissible>
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

export default async function RootLayout({ children }: { children: ReactNode }) {
  const pageMap = await getPageMap();
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      {/* primaryHue/primarySaturation from the old theme.config move onto Head. */}
      <Head color={{ hue: 135, saturation: 64 }} />
      <body>
        <Layout
          banner={banner}
          navbar={navbar}
          footer={<Footer />}
          pageMap={pageMap}
          docsRepositoryBase="https://github.com/nymtech/nym/tree/develop/documentation/docs"
          sidebar={{ defaultMenuCollapseLevel: 1, autoCollapse: true }}
          toc={{ float: false }}
          editLink={null}
          feedback={{ content: null }}
          darkMode
          nextThemes={{ defaultTheme: "dark" }}
        >
          <Providers>{children}</Providers>
        </Layout>
      </body>
    </html>
  );
}
