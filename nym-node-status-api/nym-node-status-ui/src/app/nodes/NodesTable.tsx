import { useAllNymNodes } from "@/hooks/useAllNymNodes";
import type { NymNode } from "@/hooks/useNymNodes";
import RefreshIcon from "@mui/icons-material/Refresh";
import { IconButton, Tooltip } from "@mui/material";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useMemo } from "react";

export default function NodesTable() {
  const { data, isError, isRefetching, isLoading, refetch } =
    useAllNymNodes().query;

  const columns = useMemo<MRT_ColumnDef<NymNode>[]>(
    //column definitions...
    () => [
      {
        accessorKey: "node_id",
        header: "Node Id",
        size: 25,
      },
      {
        accessorKey: "description.moniker",
        header: "Moniker",
      },
      {
        accessorKey: "identity_key",
        header: "Identity Key",
        Cell: ({ cell }) => (
          <code>{cell.getValue<string>()?.slice(0, 8)}...</code>
        ),
      },
      {
        accessorKey: "node_type",
        header: "Node Type",
      },
      {
        accessorKey: "bonded",
        header: "Bonded",
        Cell: ({ cell }) => (cell.getValue<boolean>() ? "✅" : "⛔️"),
      },
      {
        accessorKey: "geoip.country",
        header: "Country",
      },
      {
        accessorKey: "geoip.city",
        header: "City",
      },
      {
        accessorKey: "self_description.build_information.build_version",
        header: "Version",
      },
      {
        accessorKey: "self_description.declared_role.entry",
        header: "Entry gateway",
        Cell: ({ cell }) => (cell.getValue<boolean>() ? "✅" : "-"),
      },
      {
        accessorKey: "self_description.declared_role.exit",
        header: "Exit gateway",
        Cell: ({ cell }) => (cell.getValue<boolean>() ? "✅" : "-"),
      },
      {
        accessorKey: "self_description.declared_role.mixnode",
        header: "Mixnode",
        Cell: ({ cell }) => (cell.getValue<boolean>() ? "✅" : "-"),
      },
      {
        accessorKey: "self_description.declared_role.exit_ipr",
        header: "Runs IPR",
        Cell: ({ cell }) => (cell.getValue<boolean>() ? "✅" : "-"),
      },
      {
        accessorKey: "self_description.declared_role.exit_nr",
        header: "Runs SOCKS5 NR",
        Cell: ({ cell }) => (cell.getValue<boolean>() ? "✅" : "-"),
      },
      {
        accessorKey: "self_description.host_information.ip_address",
        header: "IP Address",
      },
      {
        accessorKey: "uptime",
        header: "Uptime",
      },
    ],
    [],
    //end
  );

  const table = useMaterialReactTable({
    columns,
    data: data || [],
    initialState: {
      showColumnFilters: true,
      density: "compact",
      pagination: { pageIndex: 0, pageSize: 100 },
    },
    muiToolbarAlertBannerProps: isError
      ? {
          color: "error",
          children: "Error loading data",
        }
      : undefined,
    renderTopToolbarCustomActions: () => (
      <Tooltip arrow title="Refresh Data">
        <IconButton onClick={() => refetch()}>
          <RefreshIcon />
        </IconButton>
      </Tooltip>
    ),
    rowCount: data?.length ?? 0,
    state: {
      isLoading,
      showAlertBanner: isError,
      showProgressBars: isRefetching,
    },
  });

  return <MaterialReactTable table={table} />;
}
