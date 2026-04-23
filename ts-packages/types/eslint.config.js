// types: shared preset + tsconfig.json project

const sharedConfig = require('@nymproject/eslint-config-react-typescript');

module.exports = [
  ...sharedConfig,
  {
    languageOptions: {
      parserOptions: {
        project: './tsconfig.json',
        tsconfigRootDir: __dirname,
      },
    },
  },
  {
    ignores: ['dist/**', 'node_modules/**', '*.config.js'],
  },
];
