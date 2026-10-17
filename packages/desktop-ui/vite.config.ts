import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@tauri-apps/api/core": path.resolve(__dirname, "./src/api/tauri-bridge.ts"),
      "@tauri-apps/api/event": path.resolve(__dirname, "./src/api/tauri-bridge.ts"),
      "@tauri-apps/api": path.resolve(__dirname, "./src/api/tauri-bridge.ts"),
    },
  },
  server: {
    port: 3001,
    strictPort: false,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:3000",
        changeOrigin: true,
      },
    },
  },
  build: {
    target: "es2022",
  },
});
