import path from 'path';
import type { StorybookConfig } from '@storybook/react-webpack5';

const config: StorybookConfig = {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|mjs|ts|tsx)'],
  addons: [
    '@storybook/addon-webpack5-compiler-swc',
    '@storybook/addon-a11y',
    '@storybook/addon-docs',
    '@storybook/addon-mcp',
  ],
  framework: '@storybook/react-webpack5',
  // The app resolves bare `src/...` imports and the `@assets` alias via the shared
  // `@nymproject/webpack` base config, which Storybook does not inherit — replicate it here.
  webpackFinal: async (cfg) => {
    // `main.ts` loads as an ESM module (no __dirname); Storybook runs from the wallet dir.
    const walletRoot = process.cwd();
    cfg.resolve = cfg.resolve ?? {};
    cfg.resolve.modules = [walletRoot, 'node_modules', ...(cfg.resolve.modules ?? [])];
    cfg.resolve.alias = {
      ...(cfg.resolve.alias ?? {}),
      '@assets': path.resolve(walletRoot, '../assets'),
    };
    return cfg;
  },
};
export default config;
