<script setup lang="ts">
import TopBar from "@/components/TopBar.vue";
</script>

<template>
  <v-app>
    <TopBar />
    <v-main>
      <router-view />
    </v-main>
  </v-app>
</template>
