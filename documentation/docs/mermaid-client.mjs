"use client";

// Turbopack cannot follow @theguild/remark-mermaid's wildcard export map to the
// `/mermaid` subpath that Nextra injects into every mermaid page, so
// next.config.js aliases that subpath to this file under turbopack. The component
// source is copied verbatim from @theguild/remark-mermaid/dist/mermaid.js (0.3.0):
// a re-export from the same subpath would alias back to itself.

import { jsx } from "react/jsx-runtime";
import { useEffect, useId, useRef, useState } from "react";

function useIsVisible(ref) {
  const [isIntersecting, setIsIntersecting] = useState(false);
  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        observer.disconnect();
        setIsIntersecting(true);
      }
    });
    observer.observe(ref.current);
    return () => {
      observer.disconnect();
    };
  }, [ref]);
  return isIntersecting;
}

function Mermaid({ chart }) {
  const id = useId();
  const [svg, setSvg] = useState("");
  const containerRef = useRef(null);
  const isVisible = useIsVisible(containerRef);
  useEffect(() => {
    if (!isVisible) {
      return;
    }
    const htmlElement = document.documentElement;
    const observer = new MutationObserver(renderChart);
    observer.observe(htmlElement, { attributes: true });
    renderChart();
    return () => {
      observer.disconnect();
    };
    async function renderChart() {
      const isDarkTheme =
        htmlElement.classList.contains("dark") ||
        htmlElement.attributes.getNamedItem("data-theme")?.value === "dark";
      const mermaidConfig = {
        startOnLoad: false,
        securityLevel: "loose",
        fontFamily: "inherit",
        themeCSS: "margin: 1.5rem auto 0;",
        theme: isDarkTheme ? "dark" : "default",
      };
      const { default: mermaid } = await import("mermaid");
      try {
        mermaid.initialize(mermaidConfig);
        const { svg: svg2 } = await mermaid.render(
          // strip invalid characters for `id` attribute
          id.replaceAll(":", ""),
          chart.replaceAll("\\n", "\n"),
          containerRef.current,
        );
        setSvg(svg2);
      } catch (error) {
        console.error("Error while rendering mermaid", error);
      }
    }
  }, [chart, isVisible]);
  return /* @__PURE__ */ jsx("div", {
    ref: containerRef,
    dangerouslySetInnerHTML: { __html: svg },
  });
}

export { Mermaid };
