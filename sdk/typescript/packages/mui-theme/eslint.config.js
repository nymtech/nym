const { airbnbTypescriptBaseFlat } = require('@nymproject/eslint-config-react-typescript/airbnb-typescript-base-flat');

module.exports = [
  ...airbnbTypescriptBaseFlat({
    baseDirectory: __dirname,
    project: './tsconfig.json',
    typedGlobs: ['**/*.ts', '**/*.tsx'],
  }),
  {
    ignores: ['tsconfig.json', '**/*.d.ts', 'dist/**', 'node_modules/**', '*.config.js'],
  },
];
