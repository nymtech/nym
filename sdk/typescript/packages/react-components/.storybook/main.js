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
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|ts|tsx)'],

  docs: {},

  typescript: {
    reactDocgen: 'react-docgen-typescript',
  },
};

export default config;
