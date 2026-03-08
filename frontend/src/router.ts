import { createRouter, createWebHashHistory } from "vue-router";
import AboutView from "@/views/AboutView.vue";
import ConfigurationView from "@/views/ConfigurationView.vue";
import MetricsView from "@/views/MetricsView.vue";

const router = createRouter({
  history: createWebHashHistory("/ui/"),
  routes: [
    { path: "/", component: ConfigurationView },
    { path: "/metrics", component: MetricsView },
    { path: "/about", component: AboutView },
    { path: "/:catchAll(.*)", redirect: "/" },
  ],
});

export default router;
