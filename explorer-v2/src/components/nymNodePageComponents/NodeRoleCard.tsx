"use client";

import { Chip, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import {
  fetchEpochRewards,
  fetchGatewayStatus,
  fetchObservatoryNodes,
} from "../../app/api";
import type {
  IObservatoryNode,
  LastProbeResult,
  NodeDescription,
} from "../../app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import StarRating from "../starRating/StarRating";

type Props = {
  paramId: string;
};

type NodeDescriptionNotNull = NonNullable<NodeDescription>;
type DeclaredRoleKey = keyof NodeDescriptionNotNull["declared_role"];
type RoleString = "Entry Node" | "Exit IPR Node" | "Exit NR Node" | "Mix Node";

const roleMapping: Record<DeclaredRoleKey, RoleString> = {
  entry: "Entry Node",
  exit_ipr: "Exit IPR Node",
  exit_nr: "Exit NR Node",
  mixnode: "Mix Node",
};

const getNodeRoles = (
  declaredRoles: NodeDescriptionNotNull["declared_role"],
): RoleString[] => {
  return Object.entries(declaredRoles)
    .filter(([, isActive]) => isActive)
    .map(([role]) => roleMapping[role as DeclaredRoleKey]);
};

const calculateQualityOfServiceStars = (quality: number): number => {
  switch (true) {
    case quality < 0.3:
      return 1;
    case quality < 0.5:
      return 2;
    case quality < 0.7:
      return 3;
    default:
      return 4;
  }
};

function calculateConfigScoreStars(probeResult: LastProbeResult): number {
  const { as_entry, as_exit } = probeResult.outcome;

  if (as_entry && as_exit) {
    const allResults = [
      as_entry.can_connect,
      as_entry.can_route,
      as_exit.can_connect,
      as_exit.can_route_ip_external_v4,
      as_exit.can_route_ip_external_v6,
      as_exit.can_route_ip_v4,
      as_exit.can_route_ip_v6,
    ];
    const combinedScore = allResults.filter(Boolean).length;

    switch (combinedScore) {
      case 7:
        return 4;
      case 6:
        return 3;
      case 5:
        return 2;
      default:
        return 1;
    }
  }

  if (as_entry) {
    const entryScore = [as_entry.can_connect, as_entry.can_route].filter(
      Boolean,
    ).length;

    return entryScore === 2 ? 4 : entryScore === 1 ? 2 : 1;
  }

  if (as_exit) {
    const exitScore = [
      as_exit.can_connect,
      as_exit.can_route_ip_external_v4,
      as_exit.can_route_ip_external_v6,
      as_exit.can_route_ip_v4,
      as_exit.can_route_ip_v6,
    ].filter(Boolean).length;

    switch (exitScore) {
      case 5:
        return 4;
      case 4:
        return 3;
      case 3:
        return 2;
      default:
        return 1;
    }
  }

  return 0;
}

function calculateWireguardPerformance(probeResult: LastProbeResult): number {
  const { wg, as_exit } = probeResult.outcome;

  if (!wg) return 1;

  const pingPerformance =
    (wg.ping_hosts_performance_v4 +
      wg.ping_hosts_performance_v6 +
      wg.ping_ips_performance_v4 +
      wg.ping_ips_performance_v6) /
    4;

  switch (true) {
    case wg.can_register &&
      wg.can_handshake_v4 &&
      wg.can_handshake_v6 &&
      wg.can_resolve_dns_v4 &&
      wg.can_resolve_dns_v6 &&
      pingPerformance > 0.75:
      return 4;
    case wg.can_register &&
      wg.can_handshake_v4 &&
      wg.can_handshake_v6 &&
      wg.can_resolve_dns_v4 &&
      wg.can_resolve_dns_v6 &&
      pingPerformance <= 0.75:
      return 3;
    case wg.can_register && (!wg.can_handshake_v4 || !wg.can_handshake_v6):
      return 2;
    case as_exit && (!as_exit.can_route_ip_v4 || !as_exit.can_route_ip_v6):
      return 1;
    default:
      return 1;
  }
}

export const NodeRoleCard = ({ paramId }: Props) => {
  let nodeInfo: IObservatoryNode | undefined;

  // Fetch node info
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

  if (paramId.length > 10) {
    nodeInfo = nymNodes?.find((node) => node.identity_key === paramId);
  } else {
    nodeInfo = nymNodes?.find((node) => node.node_id === Number(paramId));
  } // Extract node roles once `nodeInfo` is available
  const nodeRoles = nodeInfo
    ? getNodeRoles(nodeInfo.description.declared_role)
    : [];

  // Define whether to fetch gateway status
  const shouldFetchGatewayStatus = nodeRoles.some((role) =>
    ["Entry Node", "Exit IPR Node", "Exit NR Node"].includes(role),
  );

  // Fetch gateway status only if `shouldFetchGatewayStatus` is true
  const { data: gatewayStatus } = useQuery({
    queryKey: ["gatewayStatus", nodeInfo?.identity_key],
    queryFn: () => fetchGatewayStatus(nodeInfo?.identity_key || ""),
    enabled: !!nodeInfo?.identity_key && shouldFetchGatewayStatus, // ✅ Only fetch if needed
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  if (isLoading || isEpochLoading) {
    return (
      <ExplorerCard label="Node role & performance">
        <Skeleton variant="text" height={70} />
        <Skeleton variant="text" height={70} />
        <Skeleton variant="text" height={300} />
      </ExplorerCard>
    );
  }

  if (isError || !nymNodes || !epochRewardsData || isEpochError) {
    return (
      <ExplorerCard label="Node role & performance">
        <Typography variant="h3" sx={{ color: "pine.950" }}>
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

  const NodeRoles = nodeRoles.map((role) => (
    <Stack key={role} direction="row" gap={1}>
      <Chip key={role} label={role} size="small" />
    </Stack>
  ));

  if (!nodeInfo) return null;

  const qualityOfServiceStars = nodeInfo?.uptime
    ? calculateQualityOfServiceStars(nodeInfo.uptime)
    : gatewayStatus
      ? calculateQualityOfServiceStars(gatewayStatus.performance)
      : 1;

  const configScoreStars = gatewayStatus
    ? calculateConfigScoreStars(gatewayStatus.last_probe_result)
    : 0;

  const wireguardPerformanceStars = gatewayStatus
    ? calculateWireguardPerformance(gatewayStatus.last_probe_result)
    : 0;

  const nodeIsMixNodeOnly =
    NodeRoles.length === 1 && nodeRoles[0] === "Mix Node";

  // Function to calculate active set probability
  const getActiveSetProbability = (
    totalStake: number,
    stakeSaturationPoint: string,
  ): string => {
    const saturation = Number.parseFloat(stakeSaturationPoint);

    if (Number.isNaN(saturation) || saturation <= 0) {
      throw new Error("Invalid stake saturation point provided");
    }

    const ratio = (totalStake / saturation) * 100;

    if (ratio > 70) {
      return "High";
    }
    if (ratio >= 40 && ratio <= 70) {
      return "Medium";
    }
    return "Low";
  };
  const activeSetProb = getActiveSetProbability(
    nodeInfo.total_stake,
    epochRewardsData.interval.stake_saturation_point,
  );

  return (
    <ExplorerCard label="Node role & performance" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Role"
        value={
          <Stack direction="row" gap={1}>
            {NodeRoles}
          </Stack>
        }
      />
      <ExplorerListItem
        row
        divider
        label="Quality of service"
        value={<StarRating value={qualityOfServiceStars} />}
      />
      {!nodeIsMixNodeOnly && gatewayStatus && (
        <ExplorerListItem
          row
          divider
          label="Config score"
          value={<StarRating value={configScoreStars} />}
        />
      )}
      {!nodeIsMixNodeOnly && gatewayStatus && (
        <ExplorerListItem
          row
          divider
          label="Wireguard performance"
          value={<StarRating value={wireguardPerformanceStars} />}
        />
      )}
      <ExplorerListItem row label="Active set Prob." value={activeSetProb} />
    </ExplorerCard>
  );
};
