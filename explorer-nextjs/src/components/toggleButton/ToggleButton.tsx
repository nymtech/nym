"use client";
import { fetchCurrentEpoch } from "@/app/api";
import { Button, ButtonGroup, CircularProgress } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import { subSeconds } from "date-fns";
import { usePathname, useRouter } from "next/navigation";
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
  const [loading, setLoading] = useState<string | null>(null);
  const router = useRouter();
  const pathname = usePathname();

  const { data } = useQuery({
    enabled: true,
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: 30000,
    staleTime: 30000,
    refetchOnMount: true,
    keepPreviousData: false,
  });

  const queryClient = useQueryClient();

  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return;
      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime();
      const now = Date.now();
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus();
    const interval = setInterval(checkEpochStatus, 10000);
    return () => clearInterval(interval);
  }, [data]);

  const handleRefetch = useCallback(() => {
    queryClient.invalidateQueries();
  }, [queryClient]);

  useEffect(() => {
    if (!hasEpochStarted) return;
    handleRefetch();
    const interval = setInterval(handleRefetch, 10000);
    return () => clearInterval(interval);
  }, [hasEpochStarted, handleRefetch]);

  useEffect(() => {
    for (const option of options) {
      router.prefetch(option.link);
    }
  }, [router, options]);

  useEffect(() => {
    if (!pathname) return;

    setLoading(null);
  }, [pathname]);

  return (
    <ButtonGroup size={size}>
      {options.map((option) => (
        <Link
          href={option.link}
          key={option.label}
          sx={{ textDecoration: "none" }}
          onClick={() => setLoading(option.label)}
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
            disabled={loading === option.label}
          >
            {loading === option.label ? (
              <CircularProgress size={18} color="inherit" />
            ) : (
              option.label
            )}
          </Button>
        </Link>
      ))}
    </ButtonGroup>
  );
};
export default ExplorerButtonGroup;
