<script setup lang="ts">
import { useMetrics } from "@/composables/metrics";

const { data, isLoading, error } = useMetrics();
</script>

<template>
  <div class="p-6">
    <h1 class="mb-4 text-xl font-semibold text-text">Metrics</h1>
    <pre v-if="isLoading" class="text-text-muted">Loading...</pre>
    <pre v-else-if="error" class="text-red-600">Error: {{ error.message }}</pre>
    <pre
      v-else
      class="rounded border border-border bg-surface p-4 text-sm text-text"
      >{{ data }}</pre
    >
  </div>
</template>
