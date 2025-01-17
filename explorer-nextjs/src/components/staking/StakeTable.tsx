"use client";

import { useNymClient } from "@/hooks/useNymClient";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Box, Button, Stack, Tooltip, Typography } from "@mui/material";
import type { Delegation } from "@nymproject/contract-clients/Mixnet.types";
import { useLocalStorage } from "@uidotdev/usehooks";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import CountryFlag from "../countryFlag/CountryFlag";
import { Favorite } from "../favorite/Favorite";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import { Link } from "../muiLink";
import ConnectWallet from "../wallet/ConnectWallet";
import StakeActions from "./StakeActions";
import type { MappedNymNode, MappedNymNodes } from "./StakeTableWithAction";
import { fee } from "./schemas";

type DelegationWithNodeDetails = {
  node: MappedNymNode | undefined;
  delegation: Delegation;
};

const ColumnHeading = ({
  children,
}: {
  children: string | React.ReactNode;
}) => {
  return (
    <Typography sx={{ py: 2, textAlign: "center" }} variant="h5">
      {children}
    </Typography>
  );
};

const StakeTable = ({ nodes }: { nodes: MappedNymNodes }) => {
  const { nymClient, address } = useNymClient();
  const [delegations, setDelegations] = useState<DelegationWithNodeDetails[]>(
    [],
  );
  const [isLoading, setIsLoading] = useState(false);
  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });
  const [favorites] = useLocalStorage<string[]>("nym-node-favorites", []);

  const router = useRouter();

  useEffect(() => {
    if (!nymClient || !address) return;

    // Fetch staking data
    const fetchDelegations = async () => {
      const data = await nymClient?.getDelegatorDelegations({
        delegator: address,
      });
      return data.delegations;
    };

    // Combine delegations with node details
    const combineDelegationsWithNode = (delegations: Delegation[]) => {
      const delegationsWithNodeDetails = delegations.map((delegation) => {
        const node = nodes.find((node) => node.nodeId === delegation.node_id);

        return {
          node,
          delegation,
        };
      });

      return delegationsWithNodeDetails;
    };

    // Fetch and map delegations
    const fetchAndMapDelegations = async () => {
      const delegations = await fetchDelegations();
      const delegationsWithNodeDetails =
        combineDelegationsWithNode(delegations);
      setDelegations(delegationsWithNodeDetails);
    };

    fetchAndMapDelegations();
  }, [address, nodes, nymClient]);

  const handleUnstake = useCallback(
    async (nodeId?: number) => {
      try {
        if (!nodeId || !address) {
          return;
        }
        console.log("Unstaking node", nodeId);
        setIsLoading(true);
        await nymClient?.undelegate(
          {
            nodeId,
          },
          fee,
          `Explorer V2: Unstaking node ${nodeId}`,
        );
        setIsLoading(false);
        setInfoModalProps({
          open: true,
          title: "Success",
          message: "This operation can take up to one hour to process",
          onClose: () => setInfoModalProps({ open: false }),
        });
      } catch (e) {
        setInfoModalProps({
          open: true,
          title: "Error",
          message:
            e instanceof Error
              ? e.message
              : "An error occurred while unstaking",
          onClose: () => {
            setInfoModalProps({ open: false });
          },
        });
        setIsLoading(false);
      }
    },
    [address, nymClient],
  );

  const handleActionSelect = useCallback(
    (action: string, nodeId: number | undefined) => {
      switch (action) {
        case "unstake":
          handleUnstake(nodeId);
          break;
        default:
          break;
      }
    },
    [handleUnstake],
  );

  const columns: MRT_ColumnDef<DelegationWithNodeDetails>[] = useMemo(
    () => [
      {
        id: "name",
        header: "",
        Header: <ColumnHeading>Name</ColumnHeading>,
        accessorKey: "name",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">
              {row.original.node?.name || "-"}
            </Typography>
          </Stack>
        ),
      },
      {
        id: "node",
        header: "",
        Header: <ColumnHeading>Node</ColumnHeading>,
        accessorKey: "bondInformation.node.identity_key",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">
              {row.original.delegation?.node_id || "-"}
            </Typography>
            <Typography variant="body5">
              {row.original.node?.identity_key || "-"}
            </Typography>
          </Stack>
        ),
      },
      {
        id: "location",
        header: "Location",
        accessorKey: "location.country_name",
        Header: <ColumnHeading>Location</ColumnHeading>,
        Cell: ({ row }) =>
          row.original.node?.countryCode ? (
            <Tooltip title={row.original.node?.countryName}>
              <Box>
                <CountryFlag
                  countryCode={row.original.node?.countryCode}
                  countryName={row.original.node?.countryCode}
                />
              </Box>
            </Tooltip>
          ) : (
            "-"
          ),
      },
      {
        id: "stake",
        header: "Staked amount",
        accessorKey: "delegation.amount.amount",
        Header: <ColumnHeading>Stake</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">
            {formatBigNum(+row.original.delegation.amount.amount / 1_000_000)}{" "}
            NYM
          </Typography>
        ),
      },
      {
        id: "stakeSaturation",
        header: "Stake saturation",
        accessorKey: "stakeSaturation",
        Header: <ColumnHeading>Stake saturation</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">
            {row.original.node?.stakeSaturation || 0}%
          </Typography>
        ),
      },
      {
        id: "profitMarginPercentage",
        header: "Profit margin",
        accessorKey: "profitMarginPercentage",
        Header: <ColumnHeading>Profit margin</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">
            {row.original.node?.profitMarginPercentage || 0 * 100}%
          </Typography>
        ),
      },
      {
        id: "Favorite",
        header: "Favorite",
        accessorKey: "Favorite",
        Header: <ColumnHeading>Favorite</ColumnHeading>,
        sortingFn: "Favorite",
        Cell: ({ row }) => (
          <Favorite address={row.original.node?.owner || ""} />
        ),
      },
      {
        id: "action",
        header: "Action",
        Header: <ColumnHeading>Action</ColumnHeading>,
        Cell: ({ row }) => (
          <StakeActions
            nodeId={row.original.delegation?.node_id}
            onActionSelect={(action) => {
              handleActionSelect(action, row.original.delegation?.node_id);
            }}
          />
        ),
      },
    ],
    [handleActionSelect],
  );

  const table = useMaterialReactTable({
    columns,
    data: delegations,
    enableRowSelection: false,
    enableColumnOrdering: false,
    enableColumnActions: false,
    enableFullScreenToggle: false,
    enableHiding: false,
    paginationDisplayMode: "pages",
    renderEmptyRowsFallback: () => (
      <Stack gap={3} sx={{ p: 5 }} justifyContent="center" alignItems="center">
        <Typography variant="body3">
          You haven&apos;t staked on any nodes yet. Stake on a node to start
          earning rewnotards.
        </Typography>
        <Button variant="contained" size="large">
          <Link href="/explorer" underline="none" color="inherit">
            Stake
          </Link>
        </Button>
      </Stack>
    ),
    muiPaginationProps: {
      showRowsPerPage: false,
      SelectProps: {
        sx: {
          fontFamily: "labGrotesqueMono",
          fontSize: "14px",
        },
      },
      color: "primary",
      shape: "circular",
    },
    sortingFns: {
      Favorite: (rowA, rowB) => {
        const isFavoriteA = favorites.includes(rowA.original.owner);
        const isFavoriteB = favorites.includes(rowB.original.owner);

        // Sort favorites first
        if (isFavoriteA && !isFavoriteB) return -1;
        if (!isFavoriteA && isFavoriteB) return 1;

        // If both are favorites or neither, keep the original order
        return 0;
      },
    },
    initialState: {
      columnPinning: { right: ["Action", "Favorite"] },
    },

    muiColumnActionsButtonProps: {
      sx: {
        color: "red",
      },
      size: "small",
    },
    muiTablePaperProps: {
      elevation: 0,
    },
    muiTableHeadRowProps: {
      sx: {
        bgcolor: "background.paper",
      },
    },

    muiTableBodyCellProps: {
      sx: {
        border: "none",
      },
    },
    muiTableBodyRowProps: ({ row }) => ({
      onClick: () => {
        router.push(`/nym-node/${row.original.node?.nodeId || "not-found"}`);
      },
      hover: true,
      sx: {
        ":nth-child(odd)": {
          bgcolor: "#F3F7FB !important",
        },
        ":nth-child(even)": {
          bgcolor: "white !important",
        },
        cursor: "pointer",
      },
    }),
  });

  if (!nymClient || !address) {
    return (
      <Stack spacing={2} alignItems="center">
        <Typography variant="body4">
          Please connect your wallet to view your stake
        </Typography>
        <ConnectWallet hideAddressAndBalance />
      </Stack>
    );
  }

  return (
    <div>
      {isLoading && <Loading />}
      <InfoModal {...infoModalProps} />
      <MaterialReactTable table={table} />
    </div>
  );
};

export default StakeTable;
