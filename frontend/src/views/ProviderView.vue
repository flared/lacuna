<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useConfig, Provider } from "@/composables/config";

const route = useRoute();
const { data, isLoading, error } = useConfig();

const provider = computed((): Provider | null => {
  const key = route.params.key as string;
  const p = data.value?.[key];
  return p ?? null;
});
</script>

<template>
  <v-progress-circular v-if="isLoading" indeterminate />
  <v-alert v-else-if="error" type="error">{{ error.message }}</v-alert>
  <v-alert v-else-if="!provider" type="error">Provider not found.</v-alert>
  <v-card v-else>
    <v-card-text>
      <v-table>
        <tbody>
          <tr>
            <td>Lacuna Base Path</td>
            <td>
              <code>{{ `/${provider.key}` }}</code>
            </td>
          </tr>
          <tr>
            <td>Base URL</td>
            <td>
              <code>{{ provider.baseurl }}</code>
            </td>
          </tr>
          <tr>
            <td>Compatibility</td>
            <td>
              <v-chip
                v-for="(enabled, name) in provider.compatibility"
                v-show="enabled"
                :key="name"
                size="small"
              >
                {{ name }}
              </v-chip>
            </td>
          </tr>
        </tbody>
      </v-table>
    </v-card-text>
  </v-card>
</template>
