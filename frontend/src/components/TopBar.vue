<script setup lang="ts">
import { RouterLink } from "vue-router";
</script>

<template>
  <nav class="bg-primary px-6 py-3 shadow-sm">
    <div class="flex items-center gap-6">
      <RouterLink to="/" class="text-lg font-semibold text-white">
        Lacuna
      </RouterLink>
      <div class="flex gap-4">
        <RouterLink to="/" class="text-sm text-white/70 hover:text-white">
          Configuration
        </RouterLink>
        <RouterLink
          to="/metrics"
          class="text-sm text-white/70 hover:text-white"
        >
          Metrics
        </RouterLink>
      </div>
    </div>
  </nav>
</template>
