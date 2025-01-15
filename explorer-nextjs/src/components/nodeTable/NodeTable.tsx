"use client";

import { COSMOS_KIT_USE_CHAIN } from "@/config";
import { useNymClient } from "@/hooks/useNymClient";
import { useChain } from "@cosmos-kit/react";
import { Box, Button, Stack, Tooltip, Typography } from "@mui/material";
import { useLocalStorage } from "@uidotdev/usehooks";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useRouter } from "next/navigation";
import { useCallback, useMemo, useState } from "react";
import CountryFlag from "../countryFlag/CountryFlag";
import { Favorite } from "../favorite/Favorite";
import Loading from "../loading";
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
  return (
    <Typography sx={{ py: 2, textAlign: "center" }} variant="h5">
      {children}
    </Typography>
  );
};

const NodeTable = ({ nodes }: { nodes: MappedNymNodes }) => {
  const router = useRouter();
  const { nymClient } = useNymClient();

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

  const handleStakeOnNode = async ({
    nodeId,
    amount,
  }: {
    nodeId: number;
    amount: string;
  }) => {
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
      console.log({ tx });
      setSelectedNodeForStaking(undefined);
      setInfoModalProps({
        open: true,
        title: "Success",
        message: "This operation can take up to one hour to process",
        tx: tx?.transactionHash,

        onClose: () => setInfoModalProps({ open: false }),
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
  };

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
        id: "node",
        header: "",
        Header: <ColumnHeading>Node</ColumnHeading>,
        accessorKey: "bondInformation.node.identity_key",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.nodeId}</Typography>
            <Typography variant="body5">{row.original.identity_key}</Typography>
          </Stack>
        ),
      },
      {
        id: "qos",
        header: "Quality of Service",
        align: "center",
        accessorKey: "qos",
        Header: <ColumnHeading>Quality of Service</ColumnHeading>,
        Cell: () => <Typography variant="body4">Unavailable</Typography>,
      },
      {
        id: "location",
        header: "Location",
        accessorKey: "location.country_name",
        Header: <ColumnHeading>Location</ColumnHeading>,
        Cell: ({ row }) =>
          row.original.countryCode && row.original.countryName ? (
            <Tooltip title={row.original.countryName}>
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
        Cell: () => <Typography variant="body4">Unavailable</Typography>,
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
        header: "Favorite",
        accessorKey: "Favorite",
        Header: <ColumnHeading>Favorite</ColumnHeading>,
        sortingFn: "Favorite",
        Cell: ({ row }) => <Favorite address={row.original.owner} />,
      },
    ],
    [isWalletConnected, handleOnSelectStake],
  );
  const table = useMaterialReactTable({
    columns,
    data: nodes,
    enableRowSelection: false, //enable some features
    enableColumnOrdering: false, //enable a feature for all columns
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
