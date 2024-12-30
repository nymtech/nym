import type NymNode from "@/app/api/types";
import { NYM_NODES } from "@/app/api/urls";

const getNymNodes = async (): Promise<NymNode[]> => {
  const response = await fetch(`${NYM_NODES}`, {
    next: {
      revalidate: 900,
    },
  });
  const data = await response.json();

  return data;
};

export default getNymNodes;
