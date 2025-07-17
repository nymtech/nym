"use client";

import { fetchNodeDelegations } from "@/app/api";
import { Stack, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useRouter } from "next/navigation";
import { useMemo } from "react";
import type { NodeRewardDetails } from "../../app/api/types";

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

const getNymsFormated = (unyms: string) => {
  const balance = Number(unyms) / 1000000;
  return balance.toFixed();
};

type Props = {
  id: number;
};

const DelegationsTable = ({ id }: Props) => {
  const router = useRouter();
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  const { data: delegations = [], isError } = useQuery({
    queryKey: ["nodeDelegations", id],
    queryFn: () => fetchNodeDelegations(id),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const columns: MRT_ColumnDef<NodeRewardDetails>[] = useMemo(
    () => [
      {
        id: "height",
        header: "",
        Header: <ColumnHeading>Height</ColumnHeading>,
        accessorKey: "height",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.block_height}</Typography>
          </Stack>
        ),
      },
      {
        id: "address",
        header: "Delegation Address",
        align: "center",
        accessorKey: "address",
        Header: <ColumnHeading>Delegation Address</ColumnHeading>,
        Cell: ({ row }) => (
          <Typography variant="body4">{row.original.owner}</Typography>
        ),
      },
      {
        id: "amount",
        header: "Amount",
        accessorKey: "amount",
        Header: <ColumnHeading>Amount</ColumnHeading>,
        sortingFn: (rowA, rowB) => {
          const stakeA = Number.parseFloat(rowA.original.amount.amount);
          const stakeB = Number.parseFloat(rowB.original.amount.amount);
          return stakeA - stakeB;
        },
        Cell: ({ row }) => (
          <Typography variant="body4">
            {getNymsFormated(row.original.amount.amount)} NYM
          </Typography>
        ),
      },
    ],
    []
  );
  const table = useMaterialReactTable({
    columns,
    data: delegations,
    enableRowSelection: false, //enable some features
    enableColumnOrdering: false, //enable a feature for all columns
    enableColumnActions: false,
    enableFullScreenToggle: false,
    enableHiding: false,
    enableDensityToggle: false,

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
      columnPinning: { right: ["Amount"] },
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
        bgcolor: isDarkMode ? "background.default" : "background.paper",
      },
    },

    muiTableBodyCellProps: {
      sx: {
        border: "none",
      },
    },
    muiTableBodyRowProps: ({ row }) => ({
      onClick: () => {
        router.push(`/account/${row.original.owner}`);
      },
      hover: true,
      sx: {
        ":nth-child(even)": {
          bgcolor:
            theme.palette.mode === "dark"
              ? "#06252B !important"
              : "white !important",
        },
        ":nth-child(odd)": {
          bgcolor:
            theme.palette.mode === "dark"
              ? "#0A333B !important"
              : "#F3F7FB !important",
        },
        "&:hover": {
          backgroundColor: `${theme.palette.mode === "dark" ? "#004449" : "#E5E7EB"} !important`,
          transition: "background-color 0.2s ease",
        },
        cursor: "pointer",
      },
    }),
  });

  if (isError) return null;

  return <MaterialReactTable table={table} />;
};

export default DelegationsTable;
