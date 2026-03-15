import { useQuery } from "@tanstack/vue-query";
import * as z from "zod";
import { apiClient } from "@/apiClient";

const providerSchema = z.object({
  key: z.string(),
  name: z.string(),
  baseurl: z.string(),
  compatibility: z.record(z.string(), z.boolean()),
});

const configSchema = z.record(z.string(), providerSchema);

export type Provider = z.infer<typeof providerSchema>;
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
