import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import fonts from 'unplugin-fonts/vite'
import vuetify from "vite-plugin-vuetify";

export default defineConfig({
  base: "/ui/",
  plugins: [
    tailwindcss(),
    vue(),
    vuetify({
      styles: {
        configFile: 'src/styles/settings.scss',
      }
    }),
    fonts({
      fontsource: {
        families: [
          {
            name: 'Roboto Mono',
            weights: [400, 700],
          },
          {
            name: 'Roboto',
            weights: [100, 300, 400, 500, 700, 900],
            styles: ['normal', 'italic'],
          },
        ],
      },
    }),
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
