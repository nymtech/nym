import type { IObservatoryNode } from "../app/api/types";
import { DATA_OBSERVATORY_NODES_URL } from "../app/api/urls";

const getNymNodes = async (): Promise<IObservatoryNode[]> => {
  const response = await fetch(`${DATA_OBSERVATORY_NODES_URL}`, {
    next: {
      revalidate: 900,
    },
  });
  const data = await response.json();

  return data;
};

export default getNymNodes;
