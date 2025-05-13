import { getGatewaysOptions } from "@/client/@tanstack/react-query.gen";
import { useQueryContext } from "@/context/queryContext";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

export const useDVpnGateways = (props?: { min_node_version?: string }) => {
  const { client } = useQueryContext();
  const { min_node_version } = props || {};
  const key = "gateways";

  const query = useQuery({
    ...getGatewaysOptions({
      client,
      query: {
        min_node_version,
      },
    }),
    placeholderData: keepPreviousData,
  });

  return {
    key,
    query,
  };
};
