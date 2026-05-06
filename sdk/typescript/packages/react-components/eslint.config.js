const { airbnbTypescriptBaseFlat } = require('@nymproject/eslint-config-react-typescript/airbnb-typescript-base-flat');

module.exports = [
  ...airbnbTypescriptBaseFlat({
    baseDirectory: __dirname,
    project: './tsconfig.eslint.json',
    typedGlobs: ['**/*.ts', '**/*.tsx'],
  }),
  {
    // .storybook excluded: those .js files use JSX/optional-chaining without a matching parser.
    // Follow-up: add a Storybook-scoped ESLint slice (parser + plugin:storybook/recommended).
    ignores: ['tsconfig.json', '**/*.d.ts', '.storybook/**', 'dist/**', 'node_modules/**', '*.config.js'],
  },
];
