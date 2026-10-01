// Nextra 4 requires a root mdx-components file. It merges the docs theme's MDX
// components with any passed in. Per-page custom components (CsvTable, demos, etc.)
// are still imported directly inside each MDX file, so they do not belong here.
import { useMDXComponents as getThemeComponents } from "nextra-theme-docs";

const themeComponents = getThemeComponents();

export function useMDXComponents(components) {
  return {
    ...themeComponents,
    ...components,
  };
}
