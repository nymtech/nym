const { airbnbTypescriptBaseFlat } = require('@nymproject/eslint-config-react-typescript/airbnb-typescript-base-flat');

module.exports = [
  ...airbnbTypescriptBaseFlat({
    baseDirectory: __dirname,
    project: './tsconfig.eslint.json',
    typedGlobs: ['**/*.ts', '**/*.tsx'],
  }),
  {
    ignores: ['tsconfig.json', '**/*.d.ts', '.storybook/**', 'dist/**', 'node_modules/**', '*.config.js'],
  },
];
