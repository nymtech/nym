// nym-wallet: shared preset + tsconfig.eslint.json

const sharedConfig = require('@nymproject/eslint-config-react-typescript');

module.exports = [
  ...sharedConfig,
  {
    languageOptions: {
      parserOptions: {
        project: './tsconfig.eslint.json',
        tsconfigRootDir: __dirname,
      },
    },
  },
  {
    ignores: ['dist/**', 'node_modules/**', 'src-tauri/**', '*.config.js'],
  },
];
