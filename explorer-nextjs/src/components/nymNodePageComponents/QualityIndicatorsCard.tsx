"use client";

import type {
  GatewayStatus,
  IObservatoryNode,
  LastProbeResult,
  NodeDescription,
} from "@/app/api/types";
import { Chip, Stack } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import StarRating from "../starRating/StarRating";

interface IQualityIndicatorsCardProps {
  nodeInfo: IObservatoryNode;
}

type NodeDescriptionNotNull = NonNullable<NodeDescription>;
type DelcaredRoleKey = keyof NodeDescriptionNotNull["declared_role"];
type RoleString = "Entry Node" | "Exit IPR Node" | "Exit NR Node" | "Mix Node";

const roleMapping: Record<DelcaredRoleKey, RoleString> = {
  entry: "Entry Node",
  exit_ipr: "Exit IPR Node",
  exit_nr: "Exit NR Node",
  mixnode: "Mix Node",
};

function getNodeRoles(
  declaredRoles: NodeDescriptionNotNull["declared_role"],
): RoleString[] {
  const activeRoles = Object.entries(declaredRoles)
    .filter(([, isActive]) => isActive)
    .map(([role]) => roleMapping[role as DelcaredRoleKey]);

  return activeRoles;
}

function calculateQualityOfServiceStars(quality: number): number {
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
}

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
    const { can_connect, can_route } = as_entry;
    const entryScore = [can_connect, can_route].filter(Boolean).length;

    switch (entryScore) {
      case 2:
        return 4;
      case 1:
        return 2;
      default:
        return 1;
    }
  }

  if (as_exit) {
    const {
      can_connect,
      can_route_ip_external_v4,
      can_route_ip_external_v6,
      can_route_ip_v4,
      can_route_ip_v6,
    } = as_exit;

    const exitScore = [
      can_connect,
      can_route_ip_external_v4,
      can_route_ip_external_v6,
      can_route_ip_v4,
      can_route_ip_v6,
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

  return 0; // Default case if neither as_entry nor as_exit is present
}

function calculateWireguardPerformance(probeResult: LastProbeResult): number {
  const { wg, as_exit } = probeResult.outcome;

  if (!wg) {
    return 1; // Default to 1 star if Wireguard information is missing
  }

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
      return 1; // Default case
  }
}

export const QualityIndicatorsCard = (props: IQualityIndicatorsCardProps) => {
  const { nodeInfo } = props;

  const [gatewayStatus, setGatewayStatus] = useState<GatewayStatus>();

  const nodeRoles = getNodeRoles(nodeInfo.description.declared_role);
  const NodeRoles = nodeRoles.map((role) => (
    <Stack key={role} direction="row" gap={1}>
      <Chip key={role} label={role} size="small" />
    </Stack>
  ));

  const qualityOfServiceStars = nodeInfo?.uptime
    ? calculateQualityOfServiceStars(nodeInfo?.uptime)
    : gatewayStatus
      ? calculateQualityOfServiceStars(gatewayStatus.performance)
      : 1;

  const nodeIsMixNodeOnly =
    NodeRoles.length === 1 && nodeRoles[0] === "Mix Node";

  useEffect(() => {
    // Fetch data if the node has certain roles
    if (
      nodeRoles.includes("Entry Node") ||
      nodeRoles.includes("Exit IPR Node") ||
      nodeRoles.includes("Exit NR Node")
    ) {
      const fetchData = async () => {
        try {
          const response = await fetch(
            `https://mainnet-node-status-api.nymtech.cc/v2/gateways/${nodeInfo.identity_key}`,
          );
          const data: GatewayStatus = await response.json();
          setGatewayStatus(data);
        } catch (error) {
          console.error("Error fetching data:", error);
        }
      };

      fetchData();
    }
  }, [nodeRoles, nodeInfo.identity_key]);

  const configScoreStars = gatewayStatus
    ? calculateConfigScoreStars(gatewayStatus.last_probe_result)
    : 0;

  const wireguardPerformanceStars = gatewayStatus
    ? calculateWireguardPerformance(gatewayStatus.last_probe_result)
    : 0;

  return (
    <ExplorerCard label="Quality indicatiors" sx={{ height: "100%" }}>
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
      {!nodeIsMixNodeOnly && (
        <ExplorerListItem
          row
          divider
          label="Config score"
          value={<StarRating value={configScoreStars} />}
        />
      )}
      {!nodeIsMixNodeOnly && (
        <ExplorerListItem
          row
          divider
          label="Wireguard performance"
          value={<StarRating value={wireguardPerformanceStars} />}
        />
      )}
    </ExplorerCard>
  );
};
