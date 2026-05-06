import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  // Use relative paths for Tauri WebView compatibility
  base: './',
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  // Tauri expects a fixed port
  server: {
    port: 5173,
    strictPort: true,
  },
  // Build for Tauri - use esnext for modern browsers
  build: {
    target: 'esnext',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    // Use relative paths for Tauri WebView compatibility
    cssCodeSplit: true,
    rollupOptions: {
      output: {
        assetFileNames: 'assets/[name]-[hash].[ext]',
        chunkFileNames: 'assets/[name]-[hash].js',
        entryFileNames: 'assets/[name]-[hash].js',
        manualChunks(id) {
          if (!id.includes('node_modules')) {
            return
          }
          if (id.includes('@tauri-apps')) {
            return 'tauri'
          }
          if (id.includes('@vicons')) {
            return 'icons'
          }
          if (id.includes('@codemirror') || id.includes('codemirror') || id.includes('yaml')) {
            return 'editor'
          }
          if (id.includes('vue') || id.includes('pinia') || id.includes('vue-router')) {
            return 'vue-vendor'
          }
          if (id.includes('naive-ui')) {
            return 'naive-ui'
          }
          return 'vendor'
        },
      },
    },
  },
  // Env variables starting with TAURI_ are injected
  envPrefix: ['VITE_', 'TAURI_'],
  clearScreen: false,
})
