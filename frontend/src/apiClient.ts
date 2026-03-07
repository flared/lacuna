import ky from "ky";

export const apiClient = ky.create({
  retry: 0, // Overlaps with vueQuery's retry mechanism.
});
