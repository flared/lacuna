// Types
import type { App } from "vue";

// Plugins
import vuetify from "@/plugins/vuetify";
import { VueQueryPlugin } from "@tanstack/vue-query";
import router from "@/plugins/router";

export function registerPlugins(app: App) {
  app.use(vuetify);
  app.use(VueQueryPlugin);
  app.use(router);
}
