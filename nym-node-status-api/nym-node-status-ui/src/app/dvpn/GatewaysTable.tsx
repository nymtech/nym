import type { DVpnGateway } from "@/client";
import { useDVpnGateways } from "@/hooks/useGateways";
import RefreshIcon from "@mui/icons-material/Refresh";
import { IconButton, Tooltip } from "@mui/material";
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import dayjs from "dayjs";
import duration from "dayjs/plugin/duration";
import relativeTime from "dayjs/plugin/relativeTime";
import {
  type MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from "material-react-table";
import { useMemo } from "react";
import ReactCountryFlag from "react-country-flag";

dayjs.extend(duration);
dayjs.extend(relativeTime);

const regionNamesInEnglish = new Intl.DisplayNames(["en"], { type: "region" });

export default function GatewaysTable() {
  const { data, isError, isRefetching, isLoading, refetch } =
    useDVpnGateways().query;

  const columns = useMemo<MRT_ColumnDef<DVpnGateway>[]>(
    //column definitions...
    () => [
      {
        accessorKey: "name",
        header: "Name",
      },
      {
        accessorKey: "identity_key",
        header: "Identity Key",
        Cell: ({ cell }) => (
          <code>{cell.getValue<string>()?.slice(0, 8)}...</code>
        ),
      },
      {
        accessorKey: "location.two_letter_iso_country_code",
        header: "Country",
        Cell: ({ cell }) => {
          const value = cell.getValue<string>();
          return (
            <>
              <ReactCountryFlag countryCode={value} /> <code>{value}</code>
              <Typography ml={2} fontSize="inherit" component="span">
                {regionNamesInEnglish.of(value)}
              </Typography>
            </>
          );
        },
      },
      {
        accessorKey: "last_probe.last_updated_utc",
        header: "Last Probed At",
        Cell: ({ cell }) => {
          const parsed = dayjs(cell.getValue<string>());
          return (
            <Box display="flex" justifyContent="space-between" width="100%">
              <div>
                <code>{parsed.format()}</code>
              </div>
              <div>
                <strong>({parsed.fromNow()})</strong>
              </div>
            </Box>
          );
        },
      },
      {
        accessorKey: "build_information.build_version",
        header: "Version",
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
      pagination: {
        pageIndex: 0,
        pageSize: 100,
      },
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
    rowCount: data?.length || 0,
    state: {
      isLoading,
      showAlertBanner: isError,
      showProgressBars: isRefetching,
    },
  });

  return <MaterialReactTable table={table} />;
}
