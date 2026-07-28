import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const devHost = process.env.TAURI_DEV_HOST;
const port = Number(process.env.PORT) || 1420;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port,
    strictPort: !process.env.PORT,
    host: devHost || false,
    hmr: devHost
      ? {
          protocol: "ws",
          host: devHost,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
