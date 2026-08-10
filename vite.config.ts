import { resolve } from "node:path";

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  build: {
    // 多页构建：中英两个入口各产出一份独立的 index.html（<html lang> / <title> / meta description
    // 各自正确），而不是让 /en 复用同一份中文 HTML 壳（--content 计划 I1 / I2 修复）。
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        en: resolve(import.meta.dirname, "en/index.html"),
      },
    },
  },
});
