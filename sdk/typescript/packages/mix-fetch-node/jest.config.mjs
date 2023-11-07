import preset from 'ts-jest/presets/index.js';

/** @type {import('ts-jest').JestConfigWithTsJest} */
export default {
  ...preset.defaults,
  verbose: true,
  transform: {
    '^.+\\.(ts|tsx)$': [
      'ts-jest',
      {
        tsconfig: 'tsconfig.jest.json',
        useESM: true,
      },
    ],
  },
};
