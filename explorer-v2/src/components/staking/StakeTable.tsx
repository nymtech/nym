"use client";
import { useChain } from "@cosmos-kit/react";
import {
  Box,
  Button,
  Chip,
  Stack,
  Tooltip,
  Typography,
  useMediaQuery,
  useTheme,
} from "@mui/material";
import { useCallback, useEffect, useMemo, useState } from "react";

import type { Delegation } from "@nymproject/contract-clients/Mixnet.types";
import { useQueryClient } from "@tanstack/react-query";
import { useLocalStorage } from "@uidotdev/usehooks";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useRouter } from "next/navigation";
import usePendingEvents, {
  type PendingEvent,
} from "../../../src/hooks/useGetPendingEvents";
import { COSMOS_KIT_USE_CHAIN } from "../../config";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import CountryFlag from "../countryFlag/CountryFlag";
import { Favorite } from "../favorite/Favorite";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import { Link } from "../muiLink";
import ConnectWallet from "../wallet/ConnectWallet";
import StakeActions from "./StakeActions";
import StakeModal from "./StakeModal";
import type { MappedNymNode, MappedNymNodes } from "./StakeTableWithAction";
import { fee } from "./schemas";
import { useEnvironment } from "@/providers/EnvironmentProvider";

type DelegationWithNodeDetails = {
  node: MappedNymNode | undefined;
  delegation: Delegation;
  pendingEvent?: PendingEvent;
};

const ColumnHeading = ({
  children,
}: {
  children: string | React.ReactNode;
}) => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down("sm"));
  return (
    <Box
      sx={{
        width: isMobile ? "80px" : "unset",
        display: "flex",
        flexDirection: "column",
        justifyContent: "flex-start",
        alignItems: "baseline",
        p: 0,
      }}
    >
      <Typography
        sx={{
          py: 2,
          textAlign: "center",
          whiteSpace: isMobile ? "normal" : "unset", // Ensure text can wrap
          wordWrap: isMobile ? "break-word" : "unset", // Break long words
          overflowWrap: isMobile ? "break-word" : "unset", // Ensure text breaks inside the cell
          textTransform: "uppercase",
        }}
        variant={isMobile ? "caption" : "h5"}
      >
        {children}
      </Typography>
    </Box>
  );
};

const StakeTable = ({ nodes }: { nodes: MappedNymNodes }) => {
  const { nymClient, address, nymQueryClient } = useNymClient();
  const [delegations, setDelegations] = useState<DelegationWithNodeDetails[]>(
    []
  );
  const [isDataLoading, setIsLoading] = useState(false);
  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });
  const [selectedNodeForStaking, setSelectedNodeForStaking] = useState<{
    nodeId: number;
    identityKey: string;
  }>();
  const [favorites] = useLocalStorage<string[]>("nym-node-favorites", []);
  const { environment } = useEnvironment();
  const chain = environment === "mainnet" ? COSMOS_KIT_USE_CHAIN : "sandbox";
  const { isWalletConnected } = useChain(chain);
  const { data: pendingEvents } = usePendingEvents(nymQueryClient, address);

  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down("sm"));
  const isDarkMode = theme.palette.mode === "dark";
  const router = useRouter();
  const queryClient = useQueryClient();

  const handleRefetch = useCallback(async () => {
    await queryClient.invalidateQueries();
  }, [queryClient]);

  useEffect(() => {
    if (!nymClient || !address || !nymQueryClient) return;

    // Fetch staking data
    const fetchDelegations = async () => {
      const data = await nymClient?.getDelegatorDelegations({
        delegator: address,
      });
      return data.delegations;
    };

    // Combine delegations with node details and pending events
    const combineDelegationsWithNodeAndPendingEvents = (
      delegations: Delegation[],
      nodes: MappedNymNode[],
      pendingEvents: PendingEvent[] | undefined
    ) => {
      // Combine delegations with node details
      const delegationsWithNodeDetails = delegations.map((delegation) => {
        const node = nodes.find((node) => node.nodeId === delegation.node_id);
        const pendingEvent = pendingEvents?.find(
          (event) => event?.mixId === delegation.node_id
        );

        return {
          node,
          delegation,
          pendingEvent,
        };
      });

      // Add pending events that are not in the delegations list
      if (pendingEvents) {
        for (const e of pendingEvents) {
          if (
            e &&
            !delegationsWithNodeDetails.find(
              (item) =>
                item.node?.nodeId === e.mixId ||
                item.delegation.node_id === e.mixId
            )
          ) {
            delegationsWithNodeDetails.push({
              node: {
                name: "-",
                nodeId: e.mixId,
                identity_key: "-",
                countryCode: null,
                countryName: null,
                profitMarginPercentage: 0,
                owner: "-",
                stakeSaturation: 0,
              },
              pendingEvent: e,
              delegation: {
                amount: {
                  amount: e.amount?.amount || "0",
                  denom: "unym",
                },
                cumulative_reward_ratio: "0",
                height: 0,
                node_id: e.mixId,
                owner: "-",
              },
            });
          }
        }
      }

      return delegationsWithNodeDetails;
    };

    // Fetch and map delegations
    const fetchAndMapDelegations = async () => {
      const delegations = await fetchDelegations();
      const delegationsWithNodeDetails =
        combineDelegationsWithNodeAndPendingEvents(
          delegations,
          nodes,
          pendingEvents
        );

      setDelegations(delegationsWithNodeDetails);
    };

    fetchAndMapDelegations();
  }, [address, nodes, nymClient, nymQueryClient, pendingEvents]);

  const handleStakeOnNode = useCallback(
    async ({ nodeId, amount }: { nodeId: number; amount: string }) => {
      const amountToDelegate = (Number(amount) * 1_000_000).toString();
      const uNymFunds = [{ amount: amountToDelegate, denom: "unym" }];

      setIsLoading(true);
      setSelectedNodeForStaking(undefined);
      try {
        const tx = await nymClient?.delegate(
          { nodeId },
          fee,
          "Delegation from Nym Explorer V2",
          uNymFunds
        );
        setSelectedNodeForStaking(undefined);

        setInfoModalProps({
          open: true,
          title: "Success",
          message: "This operation can take up to one hour to process",
          tx: tx?.transactionHash,

          onClose: async () => {
            await handleRefetch();
            setInfoModalProps({ open: false });
          },
        });
      } catch (e) {
        const errorMessage =
          e instanceof Error ? e.message : "An error occurred while staking";
        setInfoModalProps({
          open: true,
          title: "Error",
          message: errorMessage,
          onClose: () => {
            setInfoModalProps({ open: false });
          },
        });
      }
      setIsLoading(false);
    },
    [nymClient, handleRefetch]
  );

  const handleOnSelectStake = useCallback(
    (nodeId: number, nodeIdentityKey: string | undefined) => {
      if (!isWalletConnected) {
        setInfoModalProps({
          open: true,
          title: "Connect Wallet",
          message: "Connect your wallet to stake",
          Action: (
            <ConnectWallet
              fullWidth
              onClick={() =>
                setInfoModalProps({
                  open: false,
                })
              }
            />
          ),
          onClose: () => setInfoModalProps({ open: false }),
        });
        return;
      }
      if (nodeIdentityKey) {
        setSelectedNodeForStaking({
          nodeId: nodeId,
          identityKey: nodeIdentityKey,
        });
      }
    },
    [isWalletConnected]
  );

  const handleUnstake = useCallback(
    async (nodeId?: number) => {
      try {
        if (!nodeId || !address) {
          return;
        }
        setIsLoading(true);
        await nymClient?.undelegate(
          {
            nodeId,
          },
          fee,
          `Explorer V2: Unstaking node ${nodeId}`
        );
        setIsLoading(false);
        await handleRefetch();
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
    [address, nymClient, handleRefetch]
  );

  const handleActionSelect = useCallback(
    (action: string, nodeId: number, nodeIdentityKey: string | undefined) => {
      switch (action) {
        case "stake":
          handleOnSelectStake(nodeId, nodeIdentityKey);
          break;
        case "unstake":
          handleUnstake(nodeId);
          break;
        default:
          break;
      }
    },
    [handleUnstake, handleOnSelectStake]
  );

  const getTooltipTitle = useCallback(
    (pending: PendingEvent) => {
      if (pending?.kind === "undelegate") {
        return "You have an undelegation pending";
      }

      if (pending?.kind === "delegate") {
        return `You have a delegation pending worth ${formatBigNum(
          +pending.amount.amount / 1_000_000
        )} NYM`;
      }

      return undefined;
    },
    [] // Add dependencies if necessary
  );

  const columns: MRT_ColumnDef<DelegationWithNodeDetails>[] = useMemo(
    () => [
      {
        id: "name",
        header: "",
        Header: <ColumnHeading>Name</ColumnHeading>,
        accessorKey: "node.name",
        Cell: ({ row }) =>
          row.original.node?.name ? (
            <Stack spacing={1}>
              <Typography variant="body4">{row.original.node.name}</Typography>
            </Stack>
          ) : (
            "-"
          ),
      },
      {
        id: "id",
        header: "",
        Header: <ColumnHeading>Node ID</ColumnHeading>,
        accessorKey: "delegation.node_id",
        size: 90,

        Cell: ({ row }) =>
          row.original.delegation?.node_id ? (
            <Typography variant="body4">
              {row.original.delegation.node_id || "-"}
            </Typography>
          ) : (
            "-"
          ),
      },
      {
        id: "identity_key",
        header: "",
        Header: <ColumnHeading>Identity Key</ColumnHeading>,
        accessorKey: "delegation.node.identity_key",
        Cell: ({ row }) =>
          row.original.node?.identity_key ? (
            <Typography variant="body4">
              <Stack spacing={1}>
                {row.original.node?.identity_key || "-"}
              </Stack>
            </Typography>
          ) : (
            "-"
          ),
      },

      {
        id: "location",
        header: "Location",
        accessorKey: "node.countryCode",
        size: 160,
        Header: <ColumnHeading>Location</ColumnHeading>,
        Cell: ({ row }) =>
          row.original.node?.countryCode && row.original.node?.countryName ? (
            <Box>
              <CountryFlag
                countryCode={row.original.node.countryCode}
                countryName={row.original.node?.countryName || ""}
              />
            </Box>
          ) : (
            "-"
          ),
      },
      {
        id: "stake",
        header: "Staked amount",
        accessorKey: "delegation.amount.amount",
        Header: <ColumnHeading>Stake</ColumnHeading>,
        size: 80,

        sortingFn: (rowA, rowB) => {
          const stakeA = Number.parseFloat(
            rowA.original.delegation.amount.amount
          );
          const stakeB = Number.parseFloat(
            rowB.original.delegation.amount.amount
          );
          return stakeA - stakeB;
        },
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
        accessorKey: "node.stakeSaturation",
        size: 200,
        Header: <ColumnHeading>Stake saturation</ColumnHeading>,
        sortingFn: (rowA, rowB) => {
          const saturationA = rowA.original.node?.stakeSaturation || 0;
          const saturationB = rowB.original.node?.stakeSaturation || 0;
          return saturationA - saturationB;
        },
        Cell: ({ row }) =>
          row.original.node?.stakeSaturation ? (
            <Typography variant="body4">
              {row.original.node.stakeSaturation}%
            </Typography>
          ) : (
            <Typography variant="body4">{0}%</Typography>
          ),
      },
      {
        id: "Favorite",
        header: "Favorite",
        accessorKey: "Favorite",
        enableColumnFilter: false,
        size: 50,

        Header: (
          <Stack direction="row" alignItems="center">
            <ColumnHeading>Fav</ColumnHeading>
          </Stack>
        ),
        sortingFn: (rowA, rowB) => {
          const isFavoriteA = favorites.includes(
            rowA.original.node?.owner || "-"
          );
          const isFavoriteB = favorites.includes(
            rowB.original.node?.owner || "-"
          );

          // Sort favorites first
          if (isFavoriteA && !isFavoriteB) return -1;
          if (!isFavoriteA && isFavoriteB) return 1;

          // If both are favorites or neither, keep the original order
          return 0;
        },
        Cell: ({ row }) => (
          <Favorite address={row.original.node?.owner || ""} />
        ),
      },
      {
        id: "action",
        header: "Action",
        Header: <ColumnHeading>Action</ColumnHeading>,
        size: 80,

        enableColumnFilter: false,
        Cell: ({ row }) => {
          return (
            <Box>
              {row.original.pendingEvent ? (
                <Tooltip
                  placement="left"
                  title={getTooltipTitle(row.original.pendingEvent)}
                  onClick={(e) => e.stopPropagation()}
                >
                  <Chip size="small" label="Pending events" />
                </Tooltip>
              ) : (
                <StakeActions
                  nodeId={row.original.delegation?.node_id}
                  nodeIdentityKey={row.original.node?.identity_key}
                  onActionSelect={(action) => {
                    handleActionSelect(
                      action,
                      row.original.delegation?.node_id,
                      row.original.node?.identity_key || undefined
                    );
                  }}
                />
              )}
            </Box>
          );
        },
      },
    ],
    [handleActionSelect, favorites, getTooltipTitle]
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
    enableDensityToggle: false,
    renderEmptyRowsFallback: () => (
      <Stack
        gap={3}
        sx={{ p: 5 }}
        justifyContent={isMobile ? "flex-start" : "center"}
        alignItems={isMobile ? "flex-start" : "center"}
      >
        <Typography variant="body3" width={isMobile ? 300 : "unset"}>
          You haven&apos;t staked on any nodes yet. Stake on a node to start
          earning rewards.
        </Typography>
        <Button
          variant="contained"
          size="large"
          onClick={(e) => e.stopPropagation()}
        >
          <Link href="/table" underline="none" color="inherit">
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
          color: isDarkMode ? "#FFFFFF" : "inherit",
        },
      },
      color: "primary",
      shape: "circular",
    },
    initialState: {
      columnPinning: isMobile ? {} : { right: ["Action", "Favorite"] },
    },

    muiColumnActionsButtonProps: {
      sx: {
        color: isDarkMode ? "#FFFFFF" : "inherit",
      },
      size: "small",
    },
    muiTablePaperProps: {
      elevation: 0,
      sx: {
        bgcolor: isDarkMode ? "#0F1720" : "background.paper",
      },
    },
    muiTableHeadRowProps: {
      sx: {
        bgcolor: isDarkMode ? "#374042" : "background.paper",
      },
    },
    muiTableHeadCellProps: {
      sx: {
        alignItems: "center",
        paddingRight: 0,
        color: isDarkMode ? "#FFFFFF" : "inherit",
      },
    },
    muiSearchTextFieldProps: {
      InputProps: {
        style: {
          color: isDarkMode ? "#475569" : "inherit",
        },
      },
      sx: {
        backgroundColor: isDarkMode ? "#374042" : "white",
        "& .MuiOutlinedInput-root": {
          color: isDarkMode ? "#475569" : "inherit",
          backgroundColor: isDarkMode ? "#374042" : "white",
        },
        "& .MuiOutlinedInput-notchedOutline": {
          borderColor: isDarkMode ? "#334155" : "inherit",
        },
        "&:hover .MuiOutlinedInput-notchedOutline": {
          borderColor: isDarkMode ? "#475569" : "inherit",
        },
      },
      variant: "outlined",
      size: "small",
    },
    muiFilterTextFieldProps: {
      InputProps: {
        sx: {
          color: isDarkMode ? "#FFFFFF" : "inherit",
        },
      },
      sx: {
        "& .MuiInputBase-root": {
          backgroundColor: isDarkMode ? "#1C2A2E" : "white",
        },
        "& .MuiInputBase-input::placeholder": {
          color: isDarkMode ? "#94A3B8" : "inherit",
          opacity: 1,
        },
        "& .MuiOutlinedInput-notchedOutline": {
          borderColor: isDarkMode ? "#334155" : "inherit",
        },
        "&:hover .MuiOutlinedInput-notchedOutline": {
          borderColor: isDarkMode ? "#475569" : "inherit",
        },
      },
      variant: "outlined",
      size: "small",
    },
    muiTableBodyCellProps: {
      sx: {
        border: "none",
        whiteSpace: "unset",
        wordBreak: "break-word",
        paddingRight: 0,
        color: isDarkMode ? "#FFFFFF" : "inherit",
      },
    },
    muiTableBodyRowProps: ({ row }) => ({
      onClick: () => {
        router.push(`/nym-node/${row.original.node?.nodeId || "not-found"}`);
      },
      hover: true,
      sx: {
        backgroundColor: isDarkMode
          ? row.index % 2 === 0
            ? "#3E4A4C !important"
            : "#374042 !important"
          : row.index % 2 === 0
            ? "#F3F7FB"
            : "white",
        "&:hover": {
          backgroundColor: `${isDarkMode ? "#2A3436" : "#E5E7EB"} !important`,
          transition: "background-color 0.2s ease",
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
      {isDataLoading && <Loading />}
      <StakeModal
        nodeId={selectedNodeForStaking?.nodeId}
        identityKey={selectedNodeForStaking?.identityKey}
        onStake={handleStakeOnNode}
        onClose={() => setSelectedNodeForStaking(undefined)}
      />
      <InfoModal {...infoModalProps} />
      <MaterialReactTable table={table} />
    </div>
  );
};

export default StakeTable;
