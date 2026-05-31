import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  base: './',
  server: {
    port: 5173,
    strictPort: true,
    warmup: {
      clientFiles: [
        './src/main.ts',
        './src/App.vue',
        './src/router/index.ts',
        './src/pages/SearchPage.vue',
      ],
    },
  },
  build: {
    outDir: 'dist-vue',
    assetsDir: 'assets',
  }
})
