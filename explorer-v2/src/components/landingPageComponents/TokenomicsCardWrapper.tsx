"use client";
import { fetchEpochRewards, fetchNoise, fetchNymPrice } from "@/app/api";
import { useQuery } from "@tanstack/react-query";
import { TokenomicsCard } from "./TokenomicsCard";
import { ConditionalCardWrapper } from "./ConditionalCardWrapper";
import { useEnvironment } from "@/providers/EnvironmentProvider";

export const TokenomicsCardWrapper = () => {
  const { environment } = useEnvironment();

  const {
    data: nymPrice,
    isLoading: isPriceLoading,
    isError: isPriceError,
  } = useQuery({
    queryKey: ["nymPrice"],
    queryFn: fetchNymPrice,
    staleTime: 10 * 60 * 1000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const {
    data: epochRewards,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: () => fetchEpochRewards(environment),
    staleTime: 10 * 60 * 1000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const {
    data: packetsAndStaking,
    isLoading: isStakingLoading,
    isError: isStakingError,
  } = useQuery({
    queryKey: ["noise"],
    queryFn: () => fetchNoise(environment),
    staleTime: 10 * 60 * 1000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Determine if the card should be visible
  const isLoading = isPriceLoading || isEpochLoading || isStakingLoading;
  const hasError = isPriceError || isEpochError || isStakingError;
  const hasData =
    nymPrice &&
    epochRewards &&
    packetsAndStaking &&
    Array.isArray(packetsAndStaking) &&
    packetsAndStaking.length >= 2;

  const isVisible = !hasError && (hasData || isLoading);

  return (
    <ConditionalCardWrapper size={{ xs: 12, sm: 6, lg: 3 }} visible={isVisible}>
      <TokenomicsCard />
    </ConditionalCardWrapper>
  );
};
