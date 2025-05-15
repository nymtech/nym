"use client";
import { useChain } from "@cosmos-kit/react";
import {
  Box,
  Button,
  Stack,
  Typography,
  useMediaQuery,
  useTheme,
  Tooltip,
} from "@mui/material";
import { useQueryClient } from "@tanstack/react-query";
import { useLocalStorage } from "@uidotdev/usehooks";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useRouter } from "next/navigation";
import { useCallback, useMemo, useState } from "react";

import { COSMOS_KIT_USE_CHAIN } from "../../config";
import { useNymClient } from "../../hooks/useNymClient";
import CountryFlag from "../countryFlag/CountryFlag";
import { Favorite } from "../favorite/Favorite";
import Loading from "../loading";
// import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import StakeModal from "../staking/StakeModal";
import { fee } from "../staking/schemas";
import ConnectWallet from "../wallet/ConnectWallet";
import type { MappedNymNode, MappedNymNodes } from "./NodeTableWithAction";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";

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

const NodeTable = ({ nodes }: { nodes: MappedNymNodes }) => {
  const router = useRouter();
  const { nymClient } = useNymClient();
  const queryClient = useQueryClient();
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down("sm"));
  const isDarkMode = theme.palette.mode === "dark";

  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });
  const [isLoading, setIsLoading] = useState(false);
  const [selectedNodeForStaking, setSelectedNodeForStaking] = useState<{
    nodeId: number;
    identityKey: string;
  }>();
  const [favorites] = useLocalStorage<string[]>("nym-node-favorites", []);
  const { isWalletConnected } = useChain(COSMOS_KIT_USE_CHAIN);

  const handleRefetch = useCallback(async () => {
    await queryClient.invalidateQueries();
  }, [queryClient]);

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
        await handleRefetch();
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
    (node: MappedNymNode) => {
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
      setSelectedNodeForStaking({
        nodeId: node.nodeId,
        identityKey: node.identity_key,
      });
    },
    [isWalletConnected]
  );

  const columns: MRT_ColumnDef<MappedNymNode>[] = useMemo(
    () => [
      {
        id: "Favorite",
        enableColumnFilter: false,
        header: "Favorite",
        accessorKey: "Favorite",
        size: 50,

        Header: (
          <Stack direction="row" alignItems="center">
            <ColumnHeading>Fav</ColumnHeading>
          </Stack>
        ),
        sortingFn: (a, b) => {
          const aIsFavorite = favorites.includes(a.original.owner);
          const bIsFavorite = favorites.includes(b.original.owner);

          if (aIsFavorite && !bIsFavorite) {
            return -1;
          }
          if (!aIsFavorite && bIsFavorite) {
            return 1;
          }
          return 0;
        },
        Cell: ({ row }) => <Favorite address={row.original.owner} />,
      },
      {
        id: "location",
        header: "Location",
        accessorKey: "countryName",
        size: 140,
        Header: <ColumnHeading>Location</ColumnHeading>,
        Cell: ({ row }) =>
          row.original.countryCode && row.original.countryName ? (
            <Box width="100%">
              <CountryFlag
                countryCode={row.original.countryCode || ""}
                countryName={row.original.countryName || ""}
              />
            </Box>
          ) : (
            "-"
          ),
      },
      {
        id: "name",
        header: "",
        size: 210,

        Header: <ColumnHeading>Node</ColumnHeading>,
        accessorKey: "name",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.name || "-"}</Typography>
            <Tooltip
              title={row.original.identity_key}
              placement="bottom"
              slotProps={{
                tooltip: {
                  sx: {
                    maxWidth: "none",
                    whiteSpace: "nowrap",
                    bgcolor: isDarkMode ? "#374042" : "#E5E7EB",
                    color: isDarkMode ? "#FFFFFF" : "#000000",
                    "& .MuiTooltip-arrow": {
                      color: isDarkMode ? "#374042" : "#E5E7EB",
                    },
                  },
                },
              }}
            >
              <Stack
                direction="row"
                alignItems="center"
                justifyContent="space-between"
                onClick={(e) => e.stopPropagation()}
                sx={{ height: "24px" }}
              >
                <Typography
                  variant="body5"
                  sx={{
                    height: "24px",
                    display: "flex",
                    alignItems: "center",
                  }}
                >
                  {row.original.identity_key.length > 17
                    ? `${row.original.identity_key.slice(0, 10)}...${row.original.identity_key.slice(-8)}`
                    : row.original.identity_key}
                </Typography>
                <CopyToClipboard text={row.original.identity_key} />
              </Stack>
            </Tooltip>
          </Stack>
        ),
      },
      {
        id: "id",
        header: "",
        Header: <ColumnHeading>Node ID</ColumnHeading>,
        accessorKey: "nodeId",
        size: 90,
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.nodeId}</Typography>
          </Stack>
        ),
      },

      {
        id: "qos",
        header: "Qlt of Service",
        align: "center",
        accessorKey: "qualityOfService",
        size: 100,
        Header: <ColumnHeading>Uptime</ColumnHeading>,
        Cell: ({ row }) => {
          const value = row.original.qualityOfService;
          let color = "#000000";

          if (value >= 80) {
            color = "#22C55E"; // green
          } else if (value >= 50) {
            color = "#F59E0B"; // amber/orange-yellow
          } else {
            color = "#EF4444"; // red
          }

          return (
            <Typography variant="body4" sx={{ color, fontWeight: 400 }}>
              {value.toFixed()}%
            </Typography>
          );
        },
      },

      {
        id: "stakeSaturation",
        header: "Stake saturation",
        accessorKey: "stakeSaturation",
        size: 120,

        Header: <ColumnHeading>Saturation</ColumnHeading>,
        Cell: ({ row }) => {
          const value = row.original.stakeSaturation;
          let color = "#000000";

          if (value > 100) {
            color = "#EF4444";
          } else if (value >= 75) {
            color = "#22C55E";
          } else if (value >= 25) {
            color = "#F59E0B";
          } else {
            color = "#EF4444";
          }

          return (
            <Typography variant="body4" sx={{ color, fontWeight: 400 }}>
              {value}%
            </Typography>
          );
        },
      },
      {
        id: "selfBond",
        header: "Self-bond",
        accessorKey: "selfBond",
        Header: <ColumnHeading>Self-bond</ColumnHeading>,
        Cell: ({ row }) => {
          const value = row.original.selfBond;
          let color = isDarkMode ? "#FFFFFF" : "#000000";

          if (value === 0) {
            color = "#EF4444";
          }

          return (
            <Typography
              variant="body4"
              sx={{ color, fontWeight: value === 0 ? 400 : 300 }}
            >
              {row.original.selfBond} NYM
            </Typography>
          );
        },
      },
      {
        id: "operatingCosts",
        header: "Operating costs",
        accessorKey: "operatingCosts",
        Header: <ColumnHeading>Operating costs</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">
            {row.original.operatingCosts} NYM
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
            {row.original.profitMarginPercentage}%
          </Typography>
        ),
      },
      {
        id: "Action",
        header: "Action",
        accessorKey: "Action",
        size: 120,

        Header: <ColumnHeading>Action</ColumnHeading>,
        hidden: !isWalletConnected,
        enableColumnFilter: false,
        Cell: ({ row }) => (
          <Button
            size="small"
            variant="outlined"
            onClick={(e) => {
              e.stopPropagation();
              handleOnSelectStake(row.original);
            }}
          >
            Stake
          </Button>
        ),
        enableSorting: false,
      },
    ],
    [isWalletConnected, handleOnSelectStake, favorites, isDarkMode]
  );
  const table = useMaterialReactTable({
    columns,
    data: nodes,
    enableRowSelection: false,
    enableColumnOrdering: false,
    enableColumnActions: false,
    enableFullScreenToggle: false,
    enableHiding: false,
    enableDensityToggle: false,
    enableFilterMatchHighlighting: true,
    paginationDisplayMode: "pages",
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
    initialState: {
      columnPinning: isMobile ? {} : { right: ["Action"] }, // No pinning on mobile
    },
    muiColumnActionsButtonProps: {
      sx: {
        color: "red",
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
        router.push(`/nym-node/${row.original.nodeId}`);
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

  return (
    <>
      {isLoading && <Loading />}

      <StakeModal
        nodeId={selectedNodeForStaking?.nodeId}
        identityKey={selectedNodeForStaking?.identityKey}
        onStake={handleStakeOnNode}
        onClose={() => setSelectedNodeForStaking(undefined)}
      />

      <InfoModal {...infoModalProps} />

      <MaterialReactTable table={table} />
    </>
  );
};

export default NodeTable;
