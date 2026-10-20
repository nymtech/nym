"use client";

// `AccordionTemplate`, which mdx-components.jsx gives to every page: the MUI
// accordion (accordion-template-mui.tsx) loads as its own chunk, only on a page
// that renders one. It renders on the server too, so its text is in the HTML.
import dynamic from "next/dynamic";

export const AccordionTemplate = dynamic(() => import("./accordion-template-mui").then((m) => m.AccordionTemplate));
