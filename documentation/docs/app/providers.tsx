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
      // ponytail: Nextra 4 may rename the content wrapper class; verify
      // ".nextra-content" still matches on the first render pass.
      if (img.tagName === "IMG" && img.closest(".nextra-content")) {
        img.classList.toggle("img-expanded");
      }
    };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, []);

  return (
    <ThemeProvider theme={muiTheme}>
      {children}
      <PageActionsMount />
      <McpPanel />
    </ThemeProvider>
  );
}
