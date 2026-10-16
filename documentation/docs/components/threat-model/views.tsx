"use client";

// The views of a threat-model scenario record that `<Snippet view>` renders
// (components/snippet.tsx). Snippet is in mdx-components.jsx, which is part of
// every page, so each view loads as its own chunk, only on a page that cites it.
// The diagram renders on the client only, as on the pages before (client-demos.tsx);
// the assessment and the card render on the server too, so their text is in the HTML.
import dynamic from "next/dynamic";

export const ScenarioDiagram = dynamic(
  () => import("./NetworkDiagram").then((m) => m.NetworkDiagram),
  { ssr: false },
);
export const ScenarioAssessment = dynamic(() =>
  import("./GenericScenarioView").then((m) => m.GenericScenarioView),
);
export const ScenarioCard = dynamic(() => import("./MetadataPanel").then((m) => m.MetadataPanel));
