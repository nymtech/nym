"use client";

// Redoc is a React-class-component library (mobx, styled-components, React-18 era). Imported
// directly into an MDX page it evaluates server-side at build (MDX pages are server
// components under the App Router) and throws "Class extends value undefined". Loading it via
// next/dynamic with ssr:false from this client wrapper keeps it browser-only. Props are
// unchanged, so pages use <RedocStandalone specUrl=... options={{...}} /> as before.
import dynamic from "next/dynamic";

export const RedocStandalone = dynamic(
  () => import("redoc").then((m) => m.RedocStandalone),
  { ssr: false },
);
