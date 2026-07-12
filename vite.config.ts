import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  // 禁用开发服务器，确保不占用端口
  server: {
    open: false,
    port: undefined,
    strictPort: false,
    host: false,
  },
  // 预览服务器也禁用
  preview: {
    open: false,
    port: undefined,
    host: false,
  },
  build: {
    // 代码分割配置
    rollupOptions: {
      output: {
        // 手动分块策略 - 使用函数形式
        manualChunks(id: string) {
          // 将 Chart.js 单独打包
          if (id.includes('chart.js')) {
            return 'charts'
          }
          // 将 Tauri API 单独打包
          if (id.includes('@tauri-apps')) {
            return 'tauri-api'
          }
          // 将 Vue 核心打包
          if (id.includes('vue') && !id.includes('node_modules/@vue')) {
            return 'vue-core'
          }
          // 将 node_modules 中的其他依赖打包到 vendor
          if (id.includes('node_modules')) {
            return 'vendor'
          }
        },
        // 控制 chunk 大小
        chunkFileNames: 'assets/js/[name]-[hash].js',
        entryFileNames: 'assets/js/[name]-[hash].js',
        assetFileNames: (assetInfo) => {
          const info = assetInfo.name || ''
          if (info.endsWith('.css')) {
            return 'assets/css/[name]-[hash][extname]'
          }
          return 'assets/[name]-[hash][extname]'
        },
      },
    },
    // 设置 chunk 大小警告限制为 1500KB（Element Plus 完整 CSS 较大）
    chunkSizeWarningLimit: 1500,
  },
})
