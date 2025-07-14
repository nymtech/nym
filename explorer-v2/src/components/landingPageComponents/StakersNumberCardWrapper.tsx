"use client";
import { fetchNSApiNodes } from "@/app/api";
import { useQuery } from "@tanstack/react-query";
import { StakersNumberCard } from "./StakersNumberCard";
import { ConditionalCardWrapper } from "./ConditionalCardWrapper";
import { useEnvironment } from "@/providers/EnvironmentProvider";

export const StakersNumberCardWrapper = () => {
  const { environment } = useEnvironment();

  const { data, isLoading, isError } = useQuery({
    queryKey: ["nsApiNodes"],
    queryFn: () => fetchNSApiNodes(environment),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Determine if the card should be visible
  const isVisible =
    !isLoading && !isError && data && Array.isArray(data) && data.length > 0;

  return (
    <ConditionalCardWrapper size={12} visible={isVisible}>
      <StakersNumberCard />
    </ConditionalCardWrapper>
  );
};
