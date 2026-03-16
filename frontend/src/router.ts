import { createRouter, createWebHashHistory, RouteRecordRaw } from "vue-router";
import AboutView from "@/views/AboutView.vue";
import ConfigurationView from "@/views/ConfigurationView.vue";
import MetricsView from "@/views/MetricsView.vue";
import ProviderView from "@/views/ProviderView.vue";
import ProvidersView from "@/views/ProvidersView.vue";

export interface RouteConfig {
  path: string;
  name?: string;
  component?: any;
  meta?: RouteMeta;
  redirect?: string;
}

export interface RouteMeta {
  title?: string;
  parent?: string;
}

export function getRoutes(): RouteConfig[] {
  return routes;
}

const routes: RouteConfig[] = [
  {
    path: "/",
    name: "providers",
    component: ProvidersView,
    meta: {
      title: "Providers",
    },
  },
  {
    path: "/provider/:key",
    name: "provider",
    component: ProviderView,
    meta: {
      parent: "providers",
    },
  },
  {
    path: "/metrics",
    name: "metrics",
    component: MetricsView,
    meta: {
      title: "Metrics",
    },
  },
  {
    path: "/configuration",
    name: "configuration",
    component: ConfigurationView,
    meta: {
      title: "Configuration",
    },
  },
  {
    path: "/about",
    name: "about",
    component: AboutView,
    meta: {
      title: "About",
    },
  },
  {
    path: "/:catchAll(.*)",
    redirect: "/",
  },
];

const router = createRouter({
  history: createWebHashHistory("/ui/"),
  routes: routes as RouteRecordRaw[],
});

export default router;
