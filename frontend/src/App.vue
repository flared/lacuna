<script setup lang="ts">
import { useConfig } from "@/composables/config";

const { data, isLoading, error } = useConfig();
</script>

<template>
  <h1>Lacuna</h1>
  <pre v-if="isLoading">Loading...</pre>
  <pre v-else-if="error">Error: {{ error.message }}</pre>
  <pre v-else>{{ JSON.stringify(data, null, 2) }}</pre>
</template>
