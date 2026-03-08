import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import vuetify from "vite-plugin-vuetify";

export default defineConfig({
  base: "/ui/",
  plugins: [
    tailwindcss(),
    vue(),
    vuetify(),
  ],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
   	// NOTE(aviau): Claude Desktop's preview mode passes the PORT environment
    // variable when the default port is not available:
    // - https://code.claude.com/docs/en/desktop#port-conflicts
    port: parseInt(process.env.PORT || "5173"),
    proxy: {
      "^(?!/ui)": "http://localhost:3000",
    },
  },
});
