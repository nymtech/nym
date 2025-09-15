"use client";
import { useEpochContext } from "@/providers/EpochProvider";
import { ConditionalCardWrapper } from "./ConditionalCardWrapper";
import { CurrentEpochCard } from "./CurrentEpochCard";

export const CurrentEpochCardWrapper = () => {
  const { data, isError, isLoading, epochStatus } = useEpochContext();

  // Determine if the card should be visible
  // Show the card if we have data and it's not in a pending state, or if we're still loading
  const isVisible =
    !isError && (data || isLoading) && epochStatus !== "pending";

  return (
    <ConditionalCardWrapper size={12} visible={isVisible}>
      <CurrentEpochCard />
    </ConditionalCardWrapper>
  );
};
