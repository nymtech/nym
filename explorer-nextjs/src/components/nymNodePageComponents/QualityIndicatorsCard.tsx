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
  if (quality < 0.3) {
    return 1;
  }
  if (quality < 0.5) {
    return 2;
  }
  if (quality < 0.7) {
    return 3;
  }
  return 4;
}

function calculateConfigScoreStars(probeResult: LastProbeResult): number {
  const { as_entry, as_exit } = probeResult.outcome;

  if (as_entry && as_exit) {
    // Combine all true/false values for as_entry and as_exit
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

    if (combinedScore === 7) {
      return 4; // 4 stars if all 7 are true
    }
    if (combinedScore === 6) {
      return 3; // 3 stars if 6 are true
    }
    if (combinedScore === 5) {
      return 2; // 2 stars if 5 are true
    }
    return 1; // 1 star if less than 5 are true
  }

  // Check if only as_entry exists and calculate stars
  if (as_entry) {
    const { can_connect, can_route } = as_entry;

    const entryScore = [can_connect, can_route].filter(Boolean).length;

    if (entryScore === 2) {
      return 4; // 4 stars if both are true
    }
    if (entryScore === 1) {
      return 2; // 2 stars if one is true
    }
    return 1; // 1 star if both are false
  }

  // Check if only as_exit exists and calculate stars
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

    if (exitScore === 5) {
      return 4; // 4 stars if all 5 are true
    }
    if (exitScore === 4) {
      return 3; // 3 stars if 4 true, 1 false
    }
    if (exitScore === 3) {
      return 2; // 2 stars if 3 true, 2 false
    }
    return 1; // 1 star if 2 true or less
  }

  // Default case if neither as_entry nor as_exit is present
  return 0; // No stars
}

export const QualityIndicatorsCard = (props: IQualityIndicatorsCardProps) => {
  const { nodeInfo } = props;

  const [gatewayProbeResult, setGatewayProbeResult] =
    useState<LastProbeResult>();

  const nodeRoles = getNodeRoles(nodeInfo.description.declared_role);
  const NodeRoles = nodeRoles.map((role) => (
    <Stack key={role} direction="row" gap={1}>
      <Chip key={role} label={role} size="small" />
    </Stack>
  ));

  const qualityOfServiceStars = nodeInfo?.uptime
    ? calculateQualityOfServiceStars(nodeInfo?.uptime)
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
          setGatewayProbeResult(data.last_probe_result);
        } catch (error) {
          console.error("Error fetching data:", error);
        }
      };

      fetchData();
    }
  }, [nodeRoles, nodeInfo.identity_key]);

  const configScoreStars = gatewayProbeResult
    ? calculateConfigScoreStars(gatewayProbeResult)
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
          label="Probe score"
          value={<StarRating value={4} />}
        />
      )}
    </ExplorerCard>
  );
};
