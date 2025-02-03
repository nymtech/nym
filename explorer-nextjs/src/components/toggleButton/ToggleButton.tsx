"use client";
import { fetchCurrentEpoch } from "@/app/api";
import { Button, ButtonGroup } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import { subSeconds } from "date-fns";
import { useCallback, useEffect, useState } from "react";
import { Link } from "../muiLink";

type Option = {
  label: string;
  isSelected: boolean;
  link: string;
};

type Options = [Option, Option];

const ExplorerButtonGroup = ({
  size = "small",
  options,
}: {
  size?: "small" | "medium" | "large";
  options: Options;
}) => {
  const [hasEpochStarted, setHasEpochStarted] = useState(false);

  console.log("hasEpochStarted :>> ", hasEpochStarted);

  // Use React Query to fetch data
  const { data } = useQuery({
    enabled: true,
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: 30000,
    staleTime: 30000,
    refetchOnMount: true, // Force UI update
    keepPreviousData: false, // Ensure new data updates UI
  });

  const queryClient = useQueryClient();

  // check surrent epoch
  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return;

      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime();

      const now = Date.now();
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus();

    const interval = setInterval(checkEpochStatus, 10000); // Check every 30s, regardless of data updates

    return () => clearInterval(interval);
  });

  const handleRefetch = useCallback(() => {
    queryClient.invalidateQueries(); // This will refetch ALL active queries
  }, [queryClient]);

  // Refetch all queries on epoch change
  useEffect(() => {
    if (!hasEpochStarted) return;

    handleRefetch();
    console.log("refetching data from toggle button :>> ");

    const interval = setInterval(handleRefetch, 10000); // refetch every 10sec after the epoch has started

    return () => clearInterval(interval);
  }, [hasEpochStarted, handleRefetch]);
  return (
    <ButtonGroup size={size}>
      {options.map((option) => (
        <Link
          href={option.link}
          key={option.label}
          sx={{ textDecoration: "none" }}
        >
          <Button
            sx={{
              color: option.isSelected
                ? "primary.contrastText"
                : "text.primary",
              "&:hover": {
                bgcolor: option.isSelected ? "primary.main" : "",
              },
              bgcolor: option.isSelected ? "primary.main" : "transparent",
            }}
            variant="outlined"
          >
            {option.label}
          </Button>
        </Link>
      ))}
    </ButtonGroup>
  );
};
export default ExplorerButtonGroup;
