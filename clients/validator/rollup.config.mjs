import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import json from '@rollup/plugin-json';
import commonjs from '@rollup/plugin-commonjs';

export default [
  {
    input: './src/index.ts',
    output: {
      dir: 'dist/nym-validator-client',
      format: 'cjs',
    },
    plugins: [resolve(), typescript(), commonjs(), json()],
  },
];
