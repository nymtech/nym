import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tsconfigPaths from 'vite-tsconfig-paths';
import svgr from 'vite-plugin-svgr';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), tsconfigPaths(), svgr({})],
  server: {
    port: 9000,
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, './main.html'),
        auth: resolve(__dirname, './index.html'),
      },
      output: {
        preserveModules: false,
      },
    },
  },
});
