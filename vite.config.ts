import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

const host = process.env.TAURI_DEV_HOST;

// Multi-page build: each Tauri window loads its own HTML entry.
// Keeps the palette bundle minimal and independent from the manager.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // Tauri handles the Rust side; don't let Vite watch it.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // Match the Rust/WebView2 target so we can use modern JS.
    target: "es2022",
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        palette: resolve(__dirname, "palette.html"),
        radial: resolve(__dirname, "radial.html"),
        colorpicker: resolve(__dirname, "colorpicker.html"),
      },
    },
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
