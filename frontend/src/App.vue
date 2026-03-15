<script setup lang="ts">
import TopBar from "@/components/TopBar.vue";
import BreadcrumbNav from "@/components/BreadcrumbNav.vue";
</script>

<template>
  <v-app>
    <TopBar />
    <v-main>
      <v-container>
        <BreadcrumbNav />
        <router-view />
      </v-container>
    </v-main>
  </v-app>
</template>
