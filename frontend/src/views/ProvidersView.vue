<script setup lang="ts">
import { useConfig } from "@/composables/config";

const { data, isLoading, error } = useConfig();
</script>

<template>
  <v-card>
    <v-card-text>
      <v-progress-circular v-if="isLoading" indeterminate />
      <v-alert v-else-if="error" type="error">{{ error.message }}</v-alert>
      <v-list v-else-if="data">
        <v-list-item
          v-for="provider in Object.values(data)"
          :key="provider.key"
          :title="provider.name"
          :to="{ name: 'provider', params: { key: provider.key } }"
        />
      </v-list>
    </v-card-text>
  </v-card>
</template>
