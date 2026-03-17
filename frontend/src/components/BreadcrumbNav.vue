<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useConfig } from "@/composables/config";
import { RouteMeta, RouteConfig, getRoutes } from "@/plugins/router";

const route = useRoute();
const { data } = useConfig();

interface BreadcrumbItem {
  title: string;
  to?: string;
  disabled?: boolean;
}

const items = computed(() => {
  const crumbs: BreadcrumbItem[] = [];

  // Add parent breadcrumb if defined
  const parentName = route.meta.parent as string | undefined;
  if (parentName) {
    const parentRoute: RouteConfig | undefined = getRoutes().find((r) => r.name === parentName);
    if (parentRoute) {
      crumbs.push({
        title: parentRoute.meta?.title ?? parentName,
        to: parentRoute.path,
      });
    }
  }

  // Add current page title
  const hasParent = crumbs.length > 0;
  const routeMeta: RouteMeta | undefined = route.meta as RouteMeta | undefined;
  const title = routeMeta?.title;
  if (title) {
    crumbs.push({ title, disabled: hasParent });
  } else if (route.name === "provider") {
    const key = route.params.key as string;
    const provider = data.value?.[key];
    crumbs.push({
      title: provider?.name ?? key,
      disabled: hasParent,
    });
  }

  return crumbs;
});
</script>

<template>
  <v-breadcrumbs v-if="items.length" :items="items" density="compact" divider="/" class="m-0 p-0">
    <template #title="{ item }">
      <h3 class="font-normal">{{ item.title }}</h3>
    </template>
  </v-breadcrumbs>
</template>
