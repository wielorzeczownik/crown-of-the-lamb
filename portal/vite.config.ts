import { fileURLToPath, URL } from 'node:url';

import { defineConfig } from 'vite';
import { viteSingleFile } from 'vite-plugin-singlefile';
import { createHtmlPlugin } from 'vite-plugin-html';
import autoprefixer from 'autoprefixer';

import { subsetPiazzolla } from './plugins/subset-font';

export default defineConfig({
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  plugins: [
    subsetPiazzolla(),
    viteSingleFile(),
    createHtmlPlugin({
      minify: true,
    }),
  ],
  css: {
    postcss: {
      plugins: [autoprefixer()],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    assetsInlineLimit: 100_000_000,
  },
});
