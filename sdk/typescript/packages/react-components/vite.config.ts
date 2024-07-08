import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tsconfigPaths from 'vite-tsconfig-paths';
import svgr from 'vite-plugin-svgr';
import dts from 'vite-plugin-dts';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  server: {
    port: 3000,
  },
  build: {
    lib: {
      // Could also be a dictionary or array of multiple entry points
      entry: [resolve(__dirname, 'lib/index.ts')],
      formats: ['es'],
    },
    rollupOptions: {
      // make sure to externalize deps that shouldn't be bundled
      // into your library
      external: ['react', 'react-dom', '@mui/material', '@mui/icons-material'],
    },
    emptyOutDir: true,
  },

  plugins: [
    react(),
    tsconfigPaths(),
    svgr(),
    dts({
      rollupTypes: true,
    }),
  ],
});
