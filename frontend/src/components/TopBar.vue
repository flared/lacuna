<script setup lang="ts">
const items = [
  { title: "Configuration", to: "/" },
  { title: "Metrics", to: "/metrics" },
  { title: "About", to: "/about" },
];
</script>

<template>
  <v-app-bar color="primary">
    <v-toolbar-title style="flex: 0 0 auto; margin-right: 16px" class="font-weight-bold text-white">
      Lacuna
    </v-toolbar-title>
    <v-btn v-for="item in items" :key="item.to" :to="item.to" variant="text">
      {{ item.title }}
    </v-btn>
    <v-spacer />
  </v-app-bar>
</template>
