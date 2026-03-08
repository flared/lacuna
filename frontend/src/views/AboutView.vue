<script setup lang="ts">
import { useInfo } from "@/composables/info";

const { data, isLoading, error } = useInfo();
</script>

<template>
  <v-container>
    <v-card title="About">
      <v-card-text>
        <v-progress-circular v-if="isLoading" indeterminate />
        <v-alert v-else-if="error" type="error">{{ error.message }}</v-alert>
        <div v-else>
          <p>
            <span class="font-weight-bold">Project Page:</span>
            <a href="https://github.com/Flared/lacuna" target="_blank" rel="noopener noreferrer">
              Flared/lacuna
            </a>
            <v-icon icon="mdi-github" size="small" />
          </p>
          <p class="mt-4">
            <span class="font-weight-bold">Version:</span>
            {{ data?.version }}
          </p>
          <h3 class="mt-4 font-weight-bold">License</h3>
          <pre class="mt-2 text-caption" style="white-space: pre-wrap">{{ data?.license }}</pre>
        </div>
      </v-card-text>
    </v-card>
  </v-container>
</template>
