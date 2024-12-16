"use client";

import { Box, Button, Stack, Tooltip, Typography } from "@mui/material";
import { useLocalStorage } from "@uidotdev/usehooks";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useCallback, useMemo } from "react";
import CountryFlag from "../countryFlag/CountryFlag";
import { Favorite, UnFavorite } from "../favorite/Favorite";
import type { MappedNymNode, MappedNymNodes } from "./NodeTableWithAction";

const ColumnHeading = ({
  children,
}: {
  children: string | React.ReactNode;
}) => {
  return (
    <Typography sx={{ py: 2 }} variant="h5">
      {children}
    </Typography>
  );
};

const NodeTable = ({ nodes }: { nodes: MappedNymNodes }) => {
  const [favorites, saveFavorites] = useLocalStorage<string[]>(
    "nym-node-favorites",
    [],
  );

  const handleFavorite = useCallback(
    (address: string) => {
      saveFavorites([...favorites, address]);
    },
    [favorites, saveFavorites],
  );

  const handleUnfavorite = useCallback(
    (address: string) => {
      saveFavorites(favorites.filter((favorite) => favorite !== address));
    },
    [favorites, saveFavorites],
  );

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
      {
        id: "Action",
        header: "Action",
        accessorKey: "Action",
        Header: <ColumnHeading>Action</ColumnHeading>,
        Cell: () => (
          <Button size="small" variant="outlined">
            Stake
          </Button>
        ),
      },
      {
        id: "Favorite",
        header: "Favorite",
        accessorKey: "Favorite",
        Header: <ColumnHeading>Favorite</ColumnHeading>,
        sortingFn: "Favorite",
        Cell: ({ row }) =>
          favorites.includes(row.original.bondInformation.node.identity_key) ? (
            <UnFavorite
              onUnfavorite={() =>
                handleUnfavorite(row.original.bondInformation.node.identity_key)
              }
            />
          ) : (
            <Favorite
              onFavorite={() =>
                handleFavorite(row.original.bondInformation.node.identity_key)
              }
            />
          ),
      },
    ],
    [favorites, handleFavorite, handleUnfavorite],
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
      Favorite: (a, b) => {
        if (favorites.includes(a.original.bondInformation.node.identity_key)) {
          return -1;
        }
        if (favorites.includes(b.original.bondInformation.node.identity_key)) {
          return 1;
        }
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
    muiTableBodyRowProps: {
      hover: false,
      sx: {
        ":nth-child(odd)": {
          bgcolor: "#F3F7FB !important",
        },
        ":nth-child(even)": {
          bgcolor: "white !important",
        },
      },
    },
  });
  return <MaterialReactTable table={table} />;
};

export default NodeTable;
