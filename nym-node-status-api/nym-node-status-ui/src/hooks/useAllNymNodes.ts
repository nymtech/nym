import { nymNodes } from "@/client/sdk.gen";
import { useQueryContext } from "@/context/queryContext";
import type { NymNode } from "@/hooks/useNymNodes";
import { useQuery } from "@tanstack/react-query";
import React from "react";

export const useAllNymNodes = () => {
  const { client } = useQueryContext();
  const key = "nym-nodes-all";

  const queryFn = React.useCallback(async (): Promise<NymNode[]> => {
    const size = 100;
    let busy = true;
    let page = 0;
    const allData = [];
    do {
      const { data, error } = await nymNodes({ client, query: { page, size } });
      if (error) throw error;

      if (data?.items) {
        allData.push(...data.items);
      }

      // keep querying until data is less than a page
      if ((data?.items.length || 0) < size) {
        busy = false;
      }
      page += 1;
    } while (busy);
    return allData;
  }, [client]);

  const query = useQuery({ queryKey: [key], queryFn });

  return {
    key,
    query,
  };
};
