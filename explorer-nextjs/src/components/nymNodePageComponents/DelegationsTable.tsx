"use client";

import { Stack, Typography } from "@mui/material";
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

const DelegationsTable = ({
  delegations,
}: {
  delegations: NodeRewardDetails[];
}) => {
  const router = useRouter();

  const columns: MRT_ColumnDef<NodeRewardDetails>[] = useMemo(
    () => [
      {
        id: "height",
        header: "",
        Header: <ColumnHeading>Height</ColumnHeading>,
        accessorKey: "height",
        Cell: ({ row }) => (
          <Stack spacing={1}>
            <Typography variant="body4">{row.original.height}</Typography>
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
        Cell: ({ row }) => (
          <Typography variant="body4">
            {getNymsFormated(row.original.amount.amount)} NYM
          </Typography>
        ),
      },
    ],
    [],
  );
  const table = useMaterialReactTable({
    columns,
    data: delegations,
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
        router.push(`/account/${row.original.owner}`);
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
  return <MaterialReactTable table={table} />;
};

export default DelegationsTable;
