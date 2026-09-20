import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';

// Vite 配置: 输出到 ../static/ 让 Rust 后端直接服务
export default defineConfig({
  plugins: [react()],
  root: '.',
  base: process.env.VITE_BASE ?? '/static/',
  build: {
    outDir: path.resolve(__dirname, '../static'),
    emptyOutDir: true,
    target: 'es2020',
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8080',
      '/static': 'http://localhost:8080',
    },
  },
});
