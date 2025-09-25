import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import { resolve } from 'path'
import { analyzer } from 'vite-bundle-analyzer'

export default defineConfig(({ mode }) => ({
  plugins: [
    react(),
    // Bundle analyzer - only in development to avoid slowing down builds
    ...(mode === 'development' ? [analyzer({ 
      analyzerMode: 'server',
      openAnalyzer: false // Set to true if you want it to auto-open
    })] : []),
  ],
  
  // Tauri expects a fixed port, fail if that port is not available
  server: {
    port: 5173,
    strictPort: true,
    // Enhanced HMR for faster development
    hmr: {
      overlay: false
    },
    // Pre-transform known dependencies
    fs: {
      allow: ['..']
    }
  },

  // To make use of `TAURI_DEBUG` and other env variables
  envPrefix: ['VITE_', 'TAURI_'],

  build: {
    // Tauri supports es2021, updated for 2025
    target: 'es2022',
    // Enhanced minification options
    minify: mode !== 'development' ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: mode === 'development',
    // Output directory
    outDir: 'dist',
    // Enhanced optimization
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
          three: ['three', '@react-three/fiber', '@react-three/drei', '@react-three/postprocessing'],
          ui: ['@radix-ui/react-dialog', '@radix-ui/react-dropdown-menu', '@radix-ui/react-tabs', '@radix-ui/react-tooltip'],
          utils: ['lodash-es', 'date-fns', 'zod'],
          data: ['@tanstack/react-query', '@tanstack/react-table', '@tanstack/react-virtual'],
          visualization: ['d3', '@visx/group', '@visx/scale', '@visx/shape', 'recharts'],
          audio: ['howler', 'tone'],
          animation: ['framer-motion', 'lottie-react'],
          state: ['zustand', 'valtio', 'immer']
        }
      }
    },
    // Increase chunk size limit for game assets
    chunkSizeWarningLimit: 1000
  },

  resolve: {
    alias: {
      '@': resolve('./src'),
      '@components': resolve('./src/components'),
      '@hooks': resolve('./src/hooks'),
      '@stores': resolve('./src/stores'),
      '@utils': resolve('./src/utils'),
      '@assets': resolve('./src/assets'),
      '@shaders': resolve('./src/shaders'),
      '@workers': resolve('./src/workers'),
      '@styles': resolve('./src/styles')
    }
  },

  define: {
    // Global constants
    __APP_VERSION__: JSON.stringify('0.1.0'),
    __BUILD_DATE__: JSON.stringify(new Date().toISOString()),
    // Enhanced for game development
    __DEV__: JSON.stringify(mode === 'development'),
    __PROD__: JSON.stringify(mode === 'production'),
  },

  // Enhanced dependency optimization
  optimizeDeps: {
    include: [
      'react',
      'react-dom',
      'react/jsx-runtime',
      'three',
      '@react-three/fiber',
      '@react-three/drei',
      'zustand',
      '@tauri-apps/api',
      '@tanstack/react-query',
      'framer-motion',
      'lodash-es',
      'date-fns',
      'zod'
    ],
    exclude: ['@tauri-apps/cli'],
    // Force optimization for game-specific libraries
    force: true
  },

  // Enhanced CSS processing with SCSS optimization
  css: {
    preprocessorOptions: {
      scss: {
        // Modern Sass compiler API for better performance (Vite 6.x)
        api: 'modern-compiler',
        // Global imports for your design system
        additionalData: `
          @use "@styles/base/variables" as *;
          @use "@styles/utilities/mixins" as *;
        `,
        // Enhanced error handling
        logger: {
          warn: (message: string) => console.warn(`SCSS Warning: ${message}`)
        }
      }
    },
    postcss: {
      plugins: []
    },
    // Enable CSS modules for component-specific styles
    modules: {
      localsConvention: 'camelCaseOnly'
    }
  },

  // Enhanced worker support for game computations
  worker: {
    format: 'es'
  },

  // Performance optimizations
  esbuild: {
    // Remove console logs in production
    drop: mode === 'production' ? ['console', 'debugger'] : []
  }
}))
