"use client";
import { fetchNoise } from "@/app/api";
import { useQuery } from "@tanstack/react-query";
import { ConditionalCardWrapper } from "./ConditionalCardWrapper";
import { NetworkStakeCard } from "./NetworkStakeCard";

export const NetworkStakeCardWrapper = () => {
  const { data, isLoading, isError } = useQuery({
    queryKey: ["noise"],
    queryFn: fetchNoise,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });

  // Determine if the card should be visible
  const isVisible =
    !isLoading && !isError && data && Array.isArray(data) && data.length >= 10;

  return (
    <ConditionalCardWrapper size={{ xs: 12, sm: 6, lg: 3 }} visible={isVisible}>
      <NetworkStakeCard />
    </ConditionalCardWrapper>
  );
};
