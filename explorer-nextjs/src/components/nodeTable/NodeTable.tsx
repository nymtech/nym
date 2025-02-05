"use client";
import { useChain } from "@cosmos-kit/react";
import {
  Box,
  Button,
  Stack,
  Tooltip,
  Typography,
  useMediaQuery,
  useTheme,
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
        width: "fitContent",
        maxWidth: "110px",
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
          whiteSpace: "normal", // Ensure text can wrap
          wordWrap: "break-word", // Break long words
          overflowWrap: "break-word", // Ensure text breaks inside the cell
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

  const handleRefetch = useCallback(() => {
    queryClient.invalidateQueries();
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
          uNymFunds,
        );
        setSelectedNodeForStaking(undefined);
        setInfoModalProps({
          open: true,
          title: "Success",
          message: "This operation can take up to one hour to process",
          tx: tx?.transactionHash,

          onClose: () => setInfoModalProps({ open: false }),
        });
        handleRefetch();
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
    [nymClient, handleRefetch],
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
    [isWalletConnected],
  );
  // get full country name
  const countryName = useCallback((countryCode: string) => {
    const regionNames = new Intl.DisplayNames(["en"], { type: "region" });

    return <span>{regionNames.of(countryCode)}</span>;
  }, []);

  const columns: MRT_ColumnDef<MappedNymNode>[] = useMemo(
    () => [
      {
        id: "name",
        header: "",
        Header: <ColumnHeading>Name</ColumnHeading>,
        accessorKey: "name",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.name || "-"}</Typography>
          </Stack>
        ),
      },
      {
        id: "id",
        header: "",
        Header: <ColumnHeading>ID</ColumnHeading>,
        accessorKey: "nodeId",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.nodeId}</Typography>
          </Stack>
        ),
      },
      {
        id: "identity_key",
        header: "",
        Header: <ColumnHeading>Identity Key</ColumnHeading>,
        accessorKey: "identity_key",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body5">{row.original.identity_key}</Typography>
          </Stack>
        ),
      },
      {
        id: "qos",
        header: "Quality of Service",
        align: "center",
        accessorKey: "qualityOfService",
        Header: <ColumnHeading>Quality of Service</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">
            {row.original.qualityOfService}%
          </Typography>
        ),
      },
      {
        id: "location",
        header: "Location",
        accessorKey: "countryName",
        Header: <ColumnHeading>Location</ColumnHeading>,
        Cell: ({ row }) =>
          row.original.countryCode && row.original.countryName ? (
            <Tooltip title={countryName(row.original.countryName)}>
              <Box>
                <CountryFlag
                  countryCode={row.original.countryCode || ""}
                  countryName={row.original.countryCode || ""}
                />
              </Box>
            </Tooltip>
          ) : (
            "-"
          ),
      },
      {
        id: "stakeSaturation",
        header: "Stake saturation",
        accessorKey: "stakeSaturation",
        Header: <ColumnHeading>Stake saturation</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">
            {row.original.stakeSaturation}%
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
      {
        id: "Favorite",
        enableColumnFilter: false,
        header: "Favorite",
        accessorKey: "Favorite",
        Header: (
          <Stack direction="row" alignItems="center">
            <ColumnHeading>Favorite</ColumnHeading>
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
    ],
    [isWalletConnected, handleOnSelectStake, favorites, countryName],
  );
  const table = useMaterialReactTable({
    columns,
    data: nodes,
    enableRowSelection: false,
    enableColumnOrdering: false,
    enableColumnActions: false,
    enableFullScreenToggle: false,
    enableHiding: false,
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
      columnPinning: isMobile ? {} : { right: ["Action", "Favorite"] }, // No pinning on mobile
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
    muiTableHeadCellProps: {
      sx: {
        alignItems: "center",
      },
    },

    muiTableBodyCellProps: {
      sx: {
        border: "none",
        whiteSpace: "unset", // Allow text wrapping in body cells
        wordBreak: "break-word", // Ensure long text breaks correctly
        maxWidth: "100px",
      },
    },
    muiTableBodyRowProps: ({ row }) => ({
      onClick: () => {
        router.push(`/nym-node/${row.original.nodeId}`);
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
