/**
 * plugins/vuetify.ts
 *
 * Framework documentation: https://vuetifyjs.com
 */

import { createVuetify } from "vuetify";
import "@mdi/font/css/materialdesignicons.css";
import "vuetify/styles";

export default createVuetify({
  theme: {
    defaultTheme: "light",
    themes: {
      light: {
        colors: {
          primary: "#2563eb",
          background: "#f1f5f9",
          surface: "#ffffff",
        },
        variables: {
          "theme-overlay-multiplier": 1,
        },
      },
      dark: {
        dark: true,
        colors: {
          primary: "#2563eb",
          background: "#121212",
          surface: "#1e1e1e",
        },
        variables: {
          "theme-overlay-multiplier": 1,
        },
      },
    },
  },
});
