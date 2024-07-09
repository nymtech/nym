import TsconfigPathsPlugin from 'tsconfig-paths-webpack-plugin';

export const framework = {
  name: '@storybook/react-vite',
  options: {},
};

export const docs = {
  autodocs: true,
};

export const typescript = {
  reactDocgen: 'react-docgen-typescript',
};

const config = {
  framework: '@storybook/react-vite',
  stories: ['../src/**/*.stories.mdx', '../src/**/*.stories.@(js|jsx|ts|tsx)'],
  addons: ['@storybook/addon-links', '@storybook/addon-essentials', '@storybook/addon-interactions'],
};

export default config;
