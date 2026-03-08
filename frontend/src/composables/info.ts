import { useQuery } from "@tanstack/vue-query";
import * as z from "zod";
import { apiClient } from "@/apiClient";

const infoSchema = z.object({
  version: z.string(),
  license: z.string(),
});

export type Info = z.infer<typeof infoSchema>;

async function fetchInfo(): Promise<Info> {
  const data = await apiClient.get("/api/info").json();
  return infoSchema.parse(data);
}

export function useInfo() {
  return useQuery({
    queryKey: ["info"],
    queryFn: fetchInfo,
  });
}
