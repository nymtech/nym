"use client";

import { Card, CardContent, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import DOMPurify from "isomorphic-dompurify";
import { fetchEpochRewards, fetchNSApiNodes } from "../../app/api";
import type { ExplorerData, NS_NODE } from "../../app/api/types";
import { countryName } from "../../utils/countryName";
import NodeTable from "./NodeTable";
import { useState, useEffect } from "react";
import AdvancedFilters from "./AdvancedFilters";
import { RECOMMENDED_NODES } from "@/app/constants";
import { useEnvironment } from "@/providers/EnvironmentProvider";

// Utility function to calculate node saturation point
function getNodeSaturationPoint(
  totalStake: number,
  stakeSaturationPoint: string
): number {
  const saturation = Number.parseFloat(stakeSaturationPoint);

  if (Number.isNaN(saturation) || saturation <= 0) {
    throw new Error("Invalid stake saturation point provided");
  }

  const ratio = (totalStake / saturation) * 100;

  return Number(ratio.toFixed());
}

// Map nodes with rewards data

const mappedNSApiNodes = (
  nodes: NS_NODE[],
  epochRewardsData: ExplorerData["currentEpochRewardsData"]
) =>
  nodes
    .map((node) => {
      const nodeSaturationPoint = getNodeSaturationPoint(
        +node.total_stake,
        epochRewardsData.interval.stake_saturation_point
      );

      const cleanMoniker = DOMPurify.sanitize(node.description.moniker).replace(
        /&amp;/g,
        "&"
      );

      const selfBondFormatted = node.original_pledge
        ? Number(node.original_pledge) / 1_000_000
        : 0;

      const operatingCostsFormatted = node.rewarding_details
        ? Number(
            node.rewarding_details.cost_params.interval_operating_cost.amount
          ) / 1_000_000
        : 0;

      return {
        name: cleanMoniker,
        nodeId: node.node_id,
        identity_key: node.identity_key,
        countryCode: node.geoip?.country || null,
        countryName: countryName(node.geoip?.country || null) || null,
        selfBond: selfBondFormatted,
        operatingCosts: operatingCostsFormatted,
        profitMarginPercentage: node.rewarding_details
          ? +node.rewarding_details.cost_params.profit_margin_percent * 100
          : 0,
        owner: node.bonding_address,
        stakeSaturation: nodeSaturationPoint,
        qualityOfService: +node.uptime * 100,
        mixnode: node.self_description?.declared_role.mixnode === true,
        gateway:
          node.self_description?.declared_role.entry === true ||
          node.self_description?.declared_role.exit_ipr === true ||
          node.self_description?.declared_role.exit_nr === true,
      };
    })
    .sort((a, b) => {
      // Handle null country names by putting them at the end
      if (!a.countryName && !b.countryName) return 0;
      if (!a.countryName) return 1;
      if (!b.countryName) return -1;

      // Sort alphabetically by country name
      return a.countryName.localeCompare(b.countryName);
    });

export type MappedNymNodes = ReturnType<typeof mappedNSApiNodes>;
export type MappedNymNode = MappedNymNodes[0];

const NodeTableWithAction = () => {
  // All hooks at the top!
  const [activeFilter, setActiveFilter] = useState<
    "all" | "mixnodes" | "gateways" | "recommended"
  >(() => {
    const stored = sessionStorage.getItem("nodeTableActiveFilter");
    return (
      (stored as "all" | "mixnodes" | "gateways" | "recommended") ||
      "recommended"
    );
  });
  const [uptime, setUptime] = useState<[number, number]>(() => {
    const stored = sessionStorage.getItem("nodeTableUptime");
    return stored ? JSON.parse(stored) : [0, 100];
  });
  const [saturation, setSaturation] = useState<[number, number]>([0, 100]);
  const [profitMargin, setProfitMargin] = useState<[number, number]>(() => {
    const stored = sessionStorage.getItem("nodeTableProfitMargin");
    return stored ? JSON.parse(stored) : [0, 100];
  });
  const [advancedOpen, setAdvancedOpen] = useState(() => {
    const stored = sessionStorage.getItem("nodeTableAdvancedOpen");
    return stored ? JSON.parse(stored) : false;
  });

  const { environment } = useEnvironment();

  // Wrapper functions to handle filter changes and sessionStorage
  const handleActiveFilterChange = (
    newFilter: "all" | "mixnodes" | "gateways" | "recommended"
  ) => {
    setActiveFilter(newFilter);
    sessionStorage.setItem("nodeTableActiveFilter", newFilter);
  };

  const handleUptimeChange = (newUptime: [number, number]) => {
    setUptime(newUptime);
    sessionStorage.setItem("nodeTableUptime", JSON.stringify(newUptime));
  };

  const handleSaturationChange = (newSaturation: [number, number]) => {
    setSaturation(newSaturation);
    sessionStorage.setItem(
      "nodeTableSaturation",
      JSON.stringify(newSaturation)
    );
  };

  const handleProfitMarginChange = (newProfitMargin: [number, number]) => {
    setProfitMargin(newProfitMargin);
    sessionStorage.setItem(
      "nodeTableProfitMargin",
      JSON.stringify(newProfitMargin)
    );
  };

  const handleAdvancedOpenChange = (newAdvancedOpen: boolean) => {
    setAdvancedOpen(newAdvancedOpen);
    sessionStorage.setItem(
      "nodeTableAdvancedOpen",
      JSON.stringify(newAdvancedOpen)
    );
  };

  // Use React Query to fetch epoch rewards
  const {
    data: epochRewardsData,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: fetchEpochRewards,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Use React Query to fetch Nym nodes
  const {
    data: nsApiNodes = [],
    isLoading: isNSApiNodesLoading,
    isError: isNSApiNodesError,
  } = useQuery({
    queryKey: ["nsApiNodes", environment],
    queryFn: () => fetchNSApiNodes(environment),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Map nodes with rewards data
  const nsApiNodesData = epochRewardsData
    ? mappedNSApiNodes(nsApiNodes || [], epochRewardsData)
    : [];

  // Calculate max saturation from all nodes
  const maxSaturation = Math.max(
    100,
    ...nsApiNodesData.map((n) => n.stakeSaturation || 0)
  );

  // Initialize saturation from sessionStorage or set to maxSaturation when data is loaded
  useEffect(() => {
    const stored = sessionStorage.getItem("nodeTableSaturation");
    if (stored) {
      setSaturation(JSON.parse(stored));
    } else if (nsApiNodesData.length > 0) {
      setSaturation([0, maxSaturation]);
    }
  }, [maxSaturation, nsApiNodesData.length]);

  // Calculate node counts for each type
  const nodeCounts = {
    all: nsApiNodesData.length,
    mixnodes: nsApiNodesData.filter((node) => node.mixnode).length,
    gateways: nsApiNodesData.filter((node) => node.gateway).length,
  };

  // Handle loading state
  if (isEpochLoading || isNSApiNodesLoading) {
    return (
      <Card sx={{ height: "100%", mt: 5 }}>
        <CardContent>
          <Skeleton variant="text" height={100} />
          <Skeleton variant="text" height={100} />
          <Skeleton variant="text" height={100} />
          <Skeleton variant="text" height={100} />
        </CardContent>
      </Card>
    );
  }

  // Handle error state
  if (isEpochError || isNSApiNodesError) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Error loading data. Please try again later.
        </Typography>
      </Stack>
    );
  }

  // Map nodes with rewards data
  if (!epochRewardsData) {
    return null;
  }

  // Step 1: Filter nodes by type
  const typeFilteredNodes = nsApiNodesData.filter((node) => {
    switch (activeFilter) {
      case "mixnodes":
        return node.mixnode;
      case "gateways":
        return node.gateway;
      case "recommended":
        return RECOMMENDED_NODES.includes(node.nodeId);
      default:
        return true;
    }
  });

  // Step 2: If advanced filters are open, apply them only if sliders are not at default
  const isDefault = {
    uptime: uptime[0] === 0 && uptime[1] === 100,
    saturation: saturation[0] === 0 && saturation[1] === maxSaturation,
    profitMargin: profitMargin[0] === 0 && profitMargin[1] === 100,
  };
  const filteredNodes = advancedOpen
    ? typeFilteredNodes.filter((node) => {
        const uptimeMatch =
          isDefault.uptime ||
          (node.qualityOfService >= uptime[0] &&
            node.qualityOfService <= uptime[1]);
        const saturationMatch =
          isDefault.saturation ||
          (node.stakeSaturation >= saturation[0] &&
            node.stakeSaturation <= saturation[1]);
        const profitMarginMatch =
          isDefault.profitMargin ||
          (node.profitMarginPercentage >= profitMargin[0] &&
            node.profitMarginPercentage <= profitMargin[1]);
        return uptimeMatch && saturationMatch && profitMarginMatch;
      })
    : typeFilteredNodes;

  return (
    <Stack spacing={3}>
      <AdvancedFilters
        open={advancedOpen}
        setOpen={handleAdvancedOpenChange}
        uptime={uptime}
        setUptime={handleUptimeChange}
        saturation={saturation}
        setSaturation={handleSaturationChange}
        profitMargin={profitMargin}
        setProfitMargin={handleProfitMarginChange}
        maxSaturation={maxSaturation}
        activeFilter={activeFilter}
        setActiveFilter={handleActiveFilterChange}
        nodeCounts={nodeCounts}
        environment={environment}
      />
      <NodeTable nodes={filteredNodes} />
    </Stack>
  );
};

export default NodeTableWithAction;
