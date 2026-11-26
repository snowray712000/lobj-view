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
    target: 'esnext', // WebGPU 初始化使用 top-level await
    chunkSizeWarningLimit: 1500, // three.js 本身約 850 kB
  },
});
