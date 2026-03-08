<script setup lang="ts">
import { useInfo } from "@/composables/info";

const { data, isLoading, error } = useInfo();
</script>

<template>
  <div class="p-6">
    <h1 class="mb-4 text-xl font-semibold text-text">About</h1>
    <div v-if="isLoading" class="text-text-muted">Loading...</div>
    <div v-else-if="error" class="text-red-600">Error: {{ error.message }}</div>
    <div v-else class="rounded border border-border bg-surface p-4 text-sm text-text">
      <p>
        <span class="font-semibold">Project Page:</span>
        <a
          href="https://github.com/Flared/lacuna"
          target="_blank"
          rel="noopener noreferrer"
          class="text-primary hover:text-primary-hover underline"
        >
          Flared/lacuna
        </a>
        <font-awesome-icon :icon="['fab', 'github']" />
      </p>
      <p class="mt-4">
        <span class="font-semibold">Version:</span>
        {{ data?.version }}
      </p>
      <h2 class="mt-4 font-semibold">License</h2>
      <pre class="mt-2 rounded bg-background p-3 whitespace-pre-wrap text-xs text-text-muted">{{
        data?.license
      }}</pre>
    </div>
  </div>
</template>
