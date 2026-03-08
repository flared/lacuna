<script setup lang="ts">
import { useConfig } from "@/composables/config";

const { data, isLoading, error } = useConfig();
</script>

<template>
  <v-container>
    <v-card title="Configuration">
      <v-card-text>
        <v-progress-circular v-if="isLoading" indeterminate />
        <v-alert v-else-if="error" type="error">{{ error.message }}</v-alert>
        <pre v-else class="text-body-2">{{ JSON.stringify(data, null, 2) }}</pre>
      </v-card-text>
    </v-card>
  </v-container>
</template>
