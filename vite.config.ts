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
    // Keep production CSS in the entry document. Tauri WebView startup is more
    // reliable when layout CSS is not tied to async Vue chunks.
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        assetFileNames: 'assets/[name]-[hash].[ext]',
        chunkFileNames: 'assets/[name]-[hash].js',
        entryFileNames: 'assets/[name]-[hash].js',
        manualChunks(id) {
          if (!id.includes('node_modules')) {
            return
          }
          if (id.includes('@codemirror') || id.includes('codemirror') || id.includes('yaml')) {
            return 'editor'
          }
          if (id.includes('@tauri-apps')) {
            return 'tauri'
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
