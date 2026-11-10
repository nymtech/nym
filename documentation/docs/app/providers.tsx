"use client";

// Global client-side setup that used to live in pages/_app.tsx: the MUI dark theme,
// the two client-only global widgets, and the click-to-expand handler for content
// images. Rendered inside the Nextra <Layout> in app/layout.tsx so it wraps page
// content without turning the layout itself into a client component.
import React, { useEffect, useMemo } from "react";
import dynamic from "next/dynamic";
import { ThemeProvider, createTheme } from "@mui/material/styles";

// ssr:false: both read browser-only APIs (clipboard, DOM portals).
const McpPanel = dynamic(() => import("components/McpPanel"), { ssr: false });
const PageActionsMount = dynamic(() => import("components/PageActionsMount"), {
  ssr: false,
});

export function Providers({ children }: { children: React.ReactNode }) {
  const muiTheme = useMemo(
    () =>
      createTheme({
        palette: {
          mode: "dark",
          primary: { main: "#85E89D" },
          background: { default: "#242B2D", paper: "#2A3235" },
        },
      }),
    [],
  );

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const img = e.target as HTMLElement;
      // Nextra 4 dropped the .nextra-content class; the content renders in <main>.
      if (img.tagName === "IMG" && img.closest("main")) {
        img.classList.toggle("img-expanded");
      }
    };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, []);

  // Load the Pagefind index when the search box gets focus, not on the first keystroke, so
  // the first results do not wait for the download. The theme's Search reuses window.pagefind
  // if it is set. The path and options mirror the theme's own loader.
  useEffect(() => {
    const preload = async (e: FocusEvent) => {
      if (!(e.target as HTMLElement).closest?.(".nextra-search")) return;
      document.removeEventListener("focusin", preload);
      try {
        // A variable, not a literal: the file exists only at run time, under basePath.
        const url = "/docs/_pagefind/pagefind.js";
        const pagefind = await import(/* webpackIgnore: true */ url);
        await pagefind.options({ baseUrl: "/" });
        (window as any).pagefind ??= pagefind;
        await pagefind.init();
      } catch {
        // No index (dev server before the first build): the theme shows its own notice.
      }
    };
    document.addEventListener("focusin", preload);
    return () => document.removeEventListener("focusin", preload);
  }, []);

  return (
    <ThemeProvider theme={muiTheme}>
      {children}
      <PageActionsMount />
      <McpPanel />
    </ThemeProvider>
  );
}
