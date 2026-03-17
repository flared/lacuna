<script setup lang="ts">
import { useInfo } from "@/composables/info";

const { data, isLoading, error } = useInfo();
</script>

<template>
  <v-card>
    <v-card-text>
      <v-progress-circular v-if="isLoading" indeterminate />
      <v-alert v-else-if="error" type="error">{{ error.message }}</v-alert>
      <div v-else>
        <p>
          Project Page:
          <a
            href="https://github.com/Flared/lacuna"
            target="_blank"
            rel="noopener noreferrer"
            class="text-blue-500 underline"
          >
            Flared/lacuna
          </a>
          <v-icon icon="mdi-github" size="small" />
        </p>
        <p>
          Version:
          {{ data?.version }}
        </p>
        <h3>License</h3>
        <pre>{{ data?.license }}</pre>
      </div>
    </v-card-text>
  </v-card>
</template>
