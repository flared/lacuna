import { useQuery } from "@tanstack/vue-query";
import * as z from "zod";
import { apiClient } from "@/apiClient";

const providerSchema = z.object({
  name: z.string(),
  baseurl: z.string(),
});

const configSchema = z.record(z.string(), providerSchema);

export type Config = z.infer<typeof configSchema>;

async function fetchConfig(): Promise<Config> {
  const data = await apiClient.get("/api/config").json();
  return configSchema.parse(data);
}

export function useConfig() {
  return useQuery({
    queryKey: ["config"],
    queryFn: fetchConfig,
  });
}
