import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath, URL } from "node:url";
export default defineConfig({
  root: fileURLToPath(new URL("./tests/ui", import.meta.url)),
  publicDir: "../../public",
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  build: { target: "es2021", outDir: "../../dist-ui", emptyOutDir: true },
});
