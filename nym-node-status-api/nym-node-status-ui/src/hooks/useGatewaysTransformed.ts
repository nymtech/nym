import { getGateways } from "@/client/sdk.gen";
import { useQueryContext } from "@/context/queryContext";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import React from "react";

export const useDVpnGatewaysTransformed = () => {
  const { client } = useQueryContext();
  const key = "gateways";

  const queryFn = React.useCallback(async () => {
    const { data, error } = await getGateways({ client });
    if (error) throw error;
    return (data || []).map((g) => {
      const wg = g.last_probe?.outcome.wg as any;
      const downloadSpeedMBPerSec = wg
        ? Math.round(
            (10 * ((wg?.downloaded_file_size_bytes_v4 || 0) / 1024 / 1024)) /
              ((wg?.download_duration_milliseconds_v4 || 1) / 1000),
          ) / 10
        : undefined;
      const downloadSpeedIpv6MBPerSec = wg
        ? Math.round(
            (10 * ((wg?.downloaded_file_size_bytes_v6 || 0) / 1024 / 1024)) /
              ((wg?.download_duration_milliseconds_v6 || 1) / 1000),
          ) / 10
        : undefined;
      return {
        ...g,
        extra: {
          downloadSpeedMBPerSec,
          downloadSpeedIpv6MBPerSec,
        },
      };
    });
  }, [client]);

  const query = useQuery({
    queryKey: [key],
    queryFn,
    placeholderData: keepPreviousData,
  });

  return {
    key,
    query,
  };
};
