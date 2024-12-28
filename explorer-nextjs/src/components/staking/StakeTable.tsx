"use client";

import { useNymClient } from "@/hooks/useNymClient";
import { Box, Stack, Tooltip, Typography } from "@mui/material";
import type { Delegation } from "@nymproject/contract-clients/Mixnet.types";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import CountryFlag from "../countryFlag/CountryFlag";
import ConnectWallet from "../wallet/ConnectWallet";
import type { MappedNymNode, MappedNymNodes } from "./StakeTableWithAction";

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
  const [delegations, setDelegations] = useState<Delegation[]>([]);
  const router = useRouter();

  useEffect(() => {
    if (!nymClient || !address) return;

    // Fetch staking data
    const fetchDelegations = async () => {
      const data = await nymClient?.getDelegatorDelegations({
        delegator: address,
      });
      setDelegations(data.delegations);
    };
    fetchDelegations();
  }, [address, nymClient]);

  const columns: MRT_ColumnDef<MappedNymNode>[] = useMemo(
    () => [
      {
        id: "node",
        header: "",
        Header: <ColumnHeading>Node</ColumnHeading>,
        accessorKey: "bondInformation.node.identity_key",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.nodeId}</Typography>
            <Typography variant="body5">
              {row.original.bondInformation.node.identity_key}
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
          row.original.location?.two_letter_iso_country_code ? (
            <Tooltip title={row.original.location.country_name}>
              <Box>
                <CountryFlag
                  countryCode={
                    row.original.location.two_letter_iso_country_code
                  }
                  countryName={
                    row.original.location.two_letter_iso_country_code
                  }
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
    ],
    [],
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
      Favorite: () => {
        // TODO implement sorting by favorite
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

  if (!nymClient || !address) {
    return (
      <Stack spacing={2} alignItems="center">
        <Typography variant="body4">
          Please connect your wallet to view your stake
        </Typography>
        <ConnectWallet />
      </Stack>
    );
  }

  return (
    <div>
      <MaterialReactTable table={table} />
    </div>
  );
};

export default StakeTable;
