<script setup lang="ts">
import { useTheme } from "vuetify";

const theme = useTheme();

const saved = localStorage.getItem("theme");
if (saved === "light" || saved === "dark") {
  theme.global.name.value = saved;
}

function toggleTheme() {
  const next = theme.global.current.value.dark ? "light" : "dark";
  theme.global.name.value = next;
  localStorage.setItem("theme", next);
}

const items = [
  { title: "Configuration", to: "/" },
  { title: "Metrics", to: "/metrics" },
  { title: "About", to: "/about" },
];
</script>

<template>
  <v-app-bar :color="theme.global.current.value.dark ? 'surface' : 'primary'">
    <!-- eslint-disable-next-line vue/no-restricted-static-attribute -->
    <v-toolbar-title class="toolbar-title"> Lacuna </v-toolbar-title>
    <v-btn v-for="item in items" :key="item.to" :to="item.to" variant="text">
      {{ item.title }}
    </v-btn>
    <v-spacer />
    <v-btn
      :icon="theme.global.current.value.dark ? 'mdi-weather-sunny' : 'mdi-weather-night'"
      variant="text"
      @click="toggleTheme"
    />
  </v-app-bar>
</template>

<style scoped>
.toolbar-title {
  flex: 0 0 auto;
  margin-right: 16px;
}
</style>
