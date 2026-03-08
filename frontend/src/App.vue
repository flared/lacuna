<script setup lang="ts">
import TopBar from "@/components/TopBar.vue";
</script>

<template>
  <div class="min-h-screen bg-background text-text">
    <TopBar />
    <router-view />
  </div>
</template>
