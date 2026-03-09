import { createApp } from "vue";
import { createVuetify } from "vuetify";
import { VueQueryPlugin } from "@tanstack/vue-query";
import "@mdi/font/css/materialdesignicons.css";
import App from "@/App.vue";
import router from "@/router";
import "@/theme.css";

const vuetify = createVuetify({
  theme: {
    defaultTheme: "light",
    themes: {
      light: {
        colors: {
          primary: "#2563eb",
          background: "#f1f5f9",
          surface: "#ffffff",
        },
      },
      dark: {
        dark: true,
        colors: {
          primary: "#2563eb",
          background: "#121212",
          surface: "#1e1e1e",
        },
      },
    },
  },
});

createApp(App).use(router).use(VueQueryPlugin).use(vuetify).mount("#app");
