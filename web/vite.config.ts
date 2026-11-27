import { resolve } from 'node:path';
import { defineConfig } from 'vite';

// 開發時：npm run dev（Vite, HMR）+ cargo run（API），/api 由 Vite 轉給 Rust 後端。
// 若後端因 port 被佔用而換了 port，用 LOBJ_API=http://127.0.0.1:xxxx npm run dev 指定。
const api = process.env.LOBJ_API ?? 'http://127.0.0.1:8079';

export default defineConfig({
  server: {
    proxy: { '/api': api },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // 多個入口：檢視器主畫面，與獨立的批次轉檔頁（/to-lobj2）、統計頁（/stat）、整理頁（/organize）
    rollupOptions: {
      input: { main: resolve(__dirname, 'index.html'), 'to-lobj2': resolve(__dirname, 'to-lobj2.html'), stat: resolve(__dirname, 'stat.html'), organize: resolve(__dirname, 'organize.html') },
    },
    target: 'esnext', // WebGPU 初始化使用 top-level await
    chunkSizeWarningLimit: 1500, // three.js 本身約 850 kB
  },
});
