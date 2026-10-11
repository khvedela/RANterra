import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  worker: { format: "es" },
  build: {
    target: "es2022",
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (
            id.includes("node_modules") &&
            /(@deck.gl|@luma.gl|@loaders.gl|maplibre-gl)/.test(id)
          )
            return "geography";
        },
      },
    },
  },
  server: { port: 5173, strictPort: true },
});
