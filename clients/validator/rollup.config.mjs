import typescript from '@rollup/plugin-typescript';
import nodePolyfills from 'rollup-plugin-node-polyfills';
import json from '@rollup/plugin-json';
import commonjs from '@rollup/plugin-commonjs';

export default [
  {
    input: 'src/index.ts',
    output: {
      dir: 'dist',
      format: 'cjs',
    },
    plugins: [nodePolyfills(), typescript(), commonjs(), json()],
  },
];
