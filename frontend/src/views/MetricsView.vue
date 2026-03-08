<script setup lang="ts">
import { useMetrics } from "@/composables/metrics";

const { data, isLoading, error } = useMetrics();
</script>

<template>
  <v-container>
    <v-card title="Metrics">
      <v-card-text>
        <v-progress-circular v-if="isLoading" indeterminate />
        <v-alert v-else-if="error" type="error">{{ error.message }}</v-alert>
        <pre v-else>{{ data }}</pre>
      </v-card-text>
    </v-card>
  </v-container>
</template>
