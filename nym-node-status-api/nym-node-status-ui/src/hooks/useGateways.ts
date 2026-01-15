import { getGatewaysOptions } from "@/client/@tanstack/react-query.gen";
import { useQueryContext } from "@/context/queryContext";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

export const useDVpnGateways = () => {
  const { client } = useQueryContext();
  const key = "gateways";

  const query = useQuery({
    ...getGatewaysOptions({
      client,
    }),
    placeholderData: keepPreviousData,
  });

  return {
    key,
    query,
  };
};
