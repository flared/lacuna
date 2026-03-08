<script setup lang="ts">
import { useConfig } from "@/composables/config";

const { data, isLoading, error } = useConfig();
</script>

<template>
  <div class="p-6">
    <h1 class="mb-4 text-xl font-semibold text-text">Configuration</h1>
    <pre v-if="isLoading" class="text-text-muted">Loading...</pre>
    <pre v-else-if="error" class="text-red-600">Error: {{ error.message }}</pre>
    <pre
      v-else
      class="rounded border border-border bg-surface p-4 text-sm text-text"
      >{{ JSON.stringify(data, null, 2) }}</pre
    >
  </div>
</template>
