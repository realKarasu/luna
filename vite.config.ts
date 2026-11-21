import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// Four pages: the app, the crosshair overlay, the look wheel overlay and the
// "Is Luna safe?" explainer.
export default defineConfig({
  base: "./",
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: "es2021",
    outDir: "dist",
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        crosshair: resolve(__dirname, "crosshair.html"),
        wheel: resolve(__dirname, "wheel.html"),
        fps: resolve(__dirname, "fps.html"),
        security: resolve(__dirname, "security.html"),
      },
    },
  },
});
