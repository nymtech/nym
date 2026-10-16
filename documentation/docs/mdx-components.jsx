// Nextra 4 requires a root mdx-components file. It merges the docs theme's MDX
// components with any passed in.
//
// A snippet (.nym-codex/snippets) imports nothing, so each component a snippet
// renders comes from here. The list matches `sites.docs.components` in
// .nym-codex/manifest.json, and a test checks that the two agree. Only light
// components belong here: this file is part of every page. A page that is not
// split still imports its own components (demos, CsvTable, and so on).
import { useMDXComponents as getThemeComponents } from "nextra-theme-docs";
import { Callout, Steps, Tabs } from "nextra/components";
import { LewesPending } from "components/lewes-pending";
import { NetworkTabCallout } from "components/demos/shared/NetworkTabCallout";
import { MixnetGlossary } from "components/demos/shared/MixnetGlossary";
import { Snippet } from "components/snippet";
import { snippetHref } from "components/snippet-link";

const themeComponents = getThemeComponents();
const ThemeLink = themeComponents.a;

// A link in a snippet names its target by id (`snippet:<id>`); the theme's
// link renders the resolved route, with the base path.
function SnippetAwareLink(props) {
  return <ThemeLink {...props} href={snippetHref(props.href)} />;
}

// The components a snippet may render (D18).
const snippetComponents = { Callout, Steps, Tabs, LewesPending, NetworkTabCallout, MixnetGlossary };

export function useMDXComponents(components) {
  return {
    ...themeComponents,
    ...snippetComponents,
    // A citation in a snippet body: a diagram, or a view of a threat-model record.
    // Each view loads as its own chunk (components/threat-model/views.tsx).
    Snippet,
    a: SnippetAwareLink,
    ...components,
  };
}
