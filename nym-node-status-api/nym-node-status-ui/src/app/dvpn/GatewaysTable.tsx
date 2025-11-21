import type { DVpnGateway } from "@/client";
import { ReverseScoreIcon, ScoreIcon } from "@/components/ScoreIcon";
import { useDVpnGatewaysTransformed } from "@/hooks/useGatewaysTransformed";
import RefreshIcon from "@mui/icons-material/Refresh";
import {
  IconButton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Tooltip,
} from "@mui/material";
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

const staleGatewayBinWidthMinutes = 15;

interface StaleGatewayStats {
  bins: number[];
  average: number;
  sum: number;
  count: number;
}

export default function GatewaysTable() {
  const { data, isError, isRefetching, isLoading, refetch } =
    useDVpnGatewaysTransformed().query;

  const staleGateways = useMemo(
    () =>
      (data || []).reduce(
        (acc, g) => {
          const last_updated_utc = g.last_probe
            ? dayjs(g.last_probe.last_updated_utc)
            : null;
          if (!last_updated_utc) return acc;
          const diff = dayjs().diff(last_updated_utc, "minutes");
          const bin = Math.floor(diff / staleGatewayBinWidthMinutes);
          if (!acc.bins[bin]) {
            acc.bins[bin] = 0;
          }
          acc.bins[bin] += 1;
          acc.sum += diff;
          acc.count += 1;
          acc.average = acc.sum / acc.count;
          return acc;
        },
        {
          bins: [],
          average: 0,
          sum: 0,
          count: 0,
        } as StaleGatewayStats,
      ),
    [data],
  );

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
        accessorKey: "location.region",
        header: "City / Region",
        Cell: ({ row }) => {
          return (
            <>
              <Typography ml={2} fontSize="inherit" component="span">
                {(row.original.location as any).city}/
                {(row.original.location as any).region}
              </Typography>
            </>
          );
        },
      },
      {
        accessorKey: "performance_v2.score",
        width: 20,
        header: "Score",
        Cell: ({ cell }) => {
          const value = cell.getValue<string>();
          return (
            <>
              <Typography
                ml={2}
                fontSize="inherit"
                component="span"
                display="flex"
                alignItems="center"
                gap={1}
              >
                <ScoreIcon score={value} />
                <span>{value || "-"}</span>
              </Typography>
            </>
          );
        },
      },
      {
        accessorKey: "performance_v2.load",
        width: 20,
        header: "Load",
        Cell: ({ cell }) => {
          const value = cell.getValue<string>();
          return (
            <>
              <Typography
                ml={2}
                fontSize="inherit"
                component="span"
                display="flex"
                alignItems="center"
                gap={1}
              >
                <ReverseScoreIcon score={value} />
                <span>{value || "-"}</span>
              </Typography>
            </>
          );
        },
      },
      {
        accessorKey: "extra.downloadSpeedMBPerSec",
        header: "Download Speed ipv4 (MB/sec)",
        Cell: ({ renderedCellValue, cell }) => {
          if (!cell.getValue()) {
            return null;
          }
          return (
            <Typography
              ml={2}
              fontSize="inherit"
              component="span"
              display="flex"
              justifyContent="end"
              mr={2}
            >
              {renderedCellValue} MB/sec
            </Typography>
          );
        },
      },
      {
        accessorKey: "extra.downloadSpeedIpv6MBPerSec",
        header: "Download Speed ipv6 (MB/sec)",
        Cell: ({ renderedCellValue, cell }) => {
          if (!cell.getValue()) {
            return null;
          }
          return (
            <Typography
              ml={2}
              fontSize="inherit"
              component="span"
              display="flex"
              justifyContent="end"
              mr={2}
            >
              {renderedCellValue} MB/sec
            </Typography>
          );
        },
      },
      {
        accessorKey: "last_probe.outcome.wg.ping_ips_performance_v4",
        header: "Probe pings (IPV4)",
        Cell: ({ cell }) => {
          const value = Math.floor(
            Number.parseFloat(cell.getValue<string>() || "0") * 100,
          );
          return (
            <>
              <Typography
                ml={2}
                fontSize="inherit"
                component="span"
                display="flex"
                alignItems="center"
                gap={1}
              >
                <span>{value}%</span>
              </Typography>
            </>
          );
        },
      },
      {
        accessorKey: "performance_v2.uptime_percentage_last_24_hours",
        width: 20,
        header: "Uptime",
        Cell: ({ cell, row }) => {
          const value: number =
            ((row.original as any).performance_v2
              ?.uptime_percentage_last_24_hours || 0) * 100;
          // const value = Math.floor(Number.parseFloat(cell.getValue<string>()) * 100);
          return (
            <>
              <Typography
                ml={2}
                fontSize="inherit"
                component="span"
                display="flex"
                alignItems="center"
                gap={1}
              >
                <span>{value}%</span>
              </Typography>
            </>
          );
        },
      },
      {
        accessorKey: "last_probe.outcome.wg.can_query_metadata_v4",
        header: "Can query metadata?",
        Cell: ({ cell }) => {
          const wg = cell.row.original.last_probe?.outcome.wg as any;
          const can_query_metadata_v4 = wg?.can_query_metadata_v4;
          return (
            <>
              <Typography
                ml={2}
                fontSize="inherit"
                component="span"
                display="flex"
                alignItems="center"
                gap={1}
              >
                {can_query_metadata_v4 === null ||
                  (can_query_metadata_v4 === undefined && <span>-</span>)}
                {can_query_metadata_v4 === true && <span>✅</span>}
                {can_query_metadata_v4 === false && <span>❌</span>}
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
        id: "last_probe_age",
        accessorKey: "last_probe.last_updated_utc",
        header: "Last Probed Age",
        Cell: ({ cell, row }) => {
          const value = row.original.last_probe?.last_updated_utc;
          if (!value) {
            return "-";
          }
          const parsed = dayjs(value);
          const age = dayjs().diff(parsed, "minutes");
          return <>{age} minutes</>;
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

  return (
    <>
      <MaterialReactTable table={table} />
      <h2>Gateway probe age</h2>
      <Box mb={2}>
        Average age is {Math.round(staleGateways.average * 10) / 10} minutes old
      </Box>
      <Box mb={2}>
        <Table style={{ width: "auto" }}>
          <TableHead>
            <TableRow>
              <TableCell width={150}>Age</TableCell>
              <TableCell>Gateways</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {staleGateways.bins.map((r, i) => (
              <TableRow key={`${(i + 1) * staleGatewayBinWidthMinutes}-bin`}>
                <TableCell>
                  {(i + 1) * staleGatewayBinWidthMinutes} mins old
                </TableCell>
                <TableCell>{r}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Box>
    </>
  );
}
