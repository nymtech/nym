import type { Metadata } from "next";
import type { ReactNode } from "react";
import { Head } from "nextra/components";
import { Providers } from "./providers";
import "nextra-theme-docs/style.css";
import "./styles.css";
import "./threat-model-viz.css";

// Bare root layout: it owns <html>/<body>, the theme <Head> (accent colour via its `color`
// prop, Nextra 4's replacement for the old primaryHue/primarySaturation), the global styles,
// and the client Providers (MUI theme + global widgets, from the old _app.tsx). The docs
// chrome (navbar/sidebar/footer/banner) is applied in app/(docs)/layout.tsx so it wraps only
// the docs routes, matching codex.

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://nym.com/docs";
const ogImage = `${siteUrl}/images/Nym_meta_Image.png`;

// Static, site-wide fallback SEO. Per-page title/description/canonical/JSON-LD come from
// app/(docs)/seo.ts.
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
  twitter: { card: "summary_large_image", site: "@nymproject", images: [ogImage] },
  appleWebApp: { title: "Nym docs" },
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head color={{ hue: 135, saturation: 64 }} />
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
