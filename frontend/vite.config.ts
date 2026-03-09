import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const backendPort = process.env.WEB_INTERFACE_PORT || '3000';

export default defineConfig({
  plugins: [svelte()],
  build: {
    outDir: '../static',
    emptyOutDir: true,
  },
  server: {
    // Proxy API calls to the Rust backend during development
    proxy: {
      '/api': {
        target: `http://localhost:${backendPort}`,
        changeOrigin: true,
      },
    },
  },
});
