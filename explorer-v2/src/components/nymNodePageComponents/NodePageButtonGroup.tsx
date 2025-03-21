'use client';

import { useQuery } from "@tanstack/react-query";
import { fetchObservatoryNodes } from "@/app/api";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";

type Props = {
    paramId: string;
};

// const resolveNodeId = async (paramId: string) => {
//     if (paramId.length > 10) {
//         return await fetchNodeIdByIdentityKey(paramId);
//     }
//     return Number(paramId);
// };

// const fetchResolvedNodeInfo = async (paramId: string) => {
//     const id = await resolveNodeId(paramId);
//     const node = await fetchNodeInfo(id);
//     return { node, id };
// };

export default function NodePageButtonGroup({ paramId }: Props) {
    const id = Number(paramId)
    const {
        data: nymNodes,
        isLoading,
        isError,
    } = useQuery({
        queryKey: ["nymNodes"],
        queryFn: fetchObservatoryNodes,
        staleTime: 10 * 60 * 1000, // 10 minutes
        refetchOnWindowFocus: false, // Prevents unnecessary refetching
        refetchOnReconnect: false,
        refetchOnMount: false,

    });

    if (!nymNodes || isError) return null;

    const nodeInfo = nymNodes.find((node) => node.node_id === id);

    if (!nodeInfo) return null;


    if (nodeInfo.bonding_address) return (
        <ExplorerButtonGroup
            onPage="Nym Node"
            options={[
                {
                    label: "Nym Node",
                    isSelected: true,
                    link: `/nym-node/${nodeInfo.node_id}`,
                },
                {
                    label: "Account",
                    isSelected: false,
                    link: `/account/${nodeInfo.bonding_address}`,
                },
            ]}
        />
    );

}
