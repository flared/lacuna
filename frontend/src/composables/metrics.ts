import { useQuery } from "@tanstack/vue-query";
import { apiClient } from "@/apiClient";

async function fetchMetrics(): Promise<string> {
  return apiClient.get("/metrics").text();
}

export function useMetrics() {
  return useQuery({
    queryKey: ["metrics"],
    queryFn: fetchMetrics,
  });
}
