import * as React from "react";
import Box from "@mui/material/Box";
import IconButton from "@mui/material/IconButton";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Typography from "@mui/material/Typography";
import KeyboardArrowDownIcon from "@mui/icons-material/KeyboardArrowDown";
import KeyboardArrowUpIcon from "@mui/icons-material/KeyboardArrowUp";
import { Card, CardContent } from "@mui/material";
import { ExplorerStaticProgressBar } from "./ExplorerStaticProgressBar";
import {
  MultiSegmentProgressBar,
  MultiSegmentProgressBarProps,
} from "./ExplorerMultiSegmentProgressBar";
import useMediaQuery from "@mui/material/useMediaQuery";

export interface IAccontStatsRowProps {
  type: string;
  allocation: number;
  amount: number;
  value: number;
  history?: { type: string; amount: number }[];
  isLastRow?: boolean;
  progressBarColor?: string;
}

const progressBarColours = [
  "#BEF885",
  "#7FB0FF",
  "#00D17D",
  "#004650",
  "#FEECB3",
];

const Row = (props: IAccontStatsRowProps) => {
  const tablet = useMediaQuery("(min-width:700px)");

  const {
    type,
    allocation,
    amount,
    value,
    history,
    isLastRow,
    progressBarColor,
  } = props;
  const [open, setOpen] = React.useState(false);

  return (
    <React.Fragment>
      {/* Main Row */}

      {tablet ? (
        <TableRow>
          <TableCell
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            {type}
          </TableCell>
          <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            <Box>
              <Typography>{allocation}%</Typography>
              <ExplorerStaticProgressBar
                value={allocation}
                color={progressBarColor || "green"}
              />
            </Box>
          </TableCell>
          <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            {amount} NYM
          </TableCell>
          <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            $ {value}
          </TableCell>
          <TableCell
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            {history && (
              <IconButton
                aria-label="expand row"
                size="small"
                onClick={() => setOpen(!open)}
              >
                {open ? <KeyboardArrowUpIcon /> : <KeyboardArrowDownIcon />}
              </IconButton>
            )}
          </TableCell>
        </TableRow>
      ) : (
        <TableRow>
          <TableCell
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            {type}
          </TableCell>
          {/* <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            <Box>
              <Typography>{allocation}%</Typography>
              <ExplorerStaticProgressBar
                value={allocation}
                color={progressBarColor || "green"}
              />
            </Box>
          </TableCell> */}
          <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            <Typography>{amount} NYM</Typography>
            <Typography>$ {value}</Typography>
          </TableCell>

          <TableCell
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
            }}
          >
            {history && (
              <IconButton
                aria-label="expand row"
                size="small"
                onClick={() => setOpen(!open)}
              >
                {open ? <KeyboardArrowUpIcon /> : <KeyboardArrowDownIcon />}
              </IconButton>
            )}
          </TableCell>
        </TableRow>
      )}

      {/* History Rows */}
      {history &&
        open &&
        history.map((historyRow, i) => (
          <TableRow key={i}>
            <TableCell
              sx={{
                display: "flex",
                alignItems: "center",
                pl: 5,
                borderBottom: "none", // Explicitly remove border
              }}
            >
              <span style={{ marginRight: 8 }}>•</span>
              {historyRow.type}
            </TableCell>

            <TableCell
              align="right"
              sx={{
                borderBottom: "none", // Explicitly remove border
              }}
            >
              {historyRow.amount}
            </TableCell>

            <TableCell
              sx={{
                borderBottom: "none", // Explicitly remove border
              }}
            >
              {/* Any additional content */}
            </TableCell>
          </TableRow>
        ))}
    </React.Fragment>
  );
};

export interface IAccountStatsCardProps {
  rows: Array<IAccontStatsRowProps>;
}

// const progressValues = [
//   { percentage: 25, color: "#4caf50" }, // Green
//   { percentage: 35, color: "#2196f3" }, // Blue
//   { percentage: 40, color: "#ff9800" }, // Orange
// ];

export const AccountStatsCard = (props: IAccountStatsCardProps) => {
  const { rows } = props;
  const tablet = useMediaQuery("(min-width:700px)");
  const progressBarPercentages = () => {
    return rows.map((row, i) => row.allocation);
  };
  const getProgressValues = () => {
    const percentages = progressBarPercentages();
    const result: Array<{ percentage: number; color: string }> = [];
    percentages.map((value, i) => {
      result.push({
        percentage: value,
        color: progressBarColours[i],
      });
    });
    return result;
  };

  const progressValues = getProgressValues();

  return (
    <Card sx={{ height: "100%", borderRadius: "unset" }}>
      <CardContent>
        {!tablet && <MultiSegmentProgressBar values={progressValues} />}
        <TableContainer>
          <Table aria-label="collapsible table" sx={{ marginBottom: 3 }}>
            <TableHead>
              {tablet ? (
                <TableRow>
                  <TableCell>Type</TableCell>
                  <TableCell align="right">Allocation</TableCell>
                  <TableCell align="right">Amount</TableCell>
                  <TableCell align="right">Value</TableCell>
                  <TableCell></TableCell>
                </TableRow>
              ) : (
                <TableRow>
                  <TableCell>Type</TableCell>
                  <TableCell align="right">Amount / Value</TableCell>
                  <TableCell></TableCell>
                </TableRow>
              )}
            </TableHead>
            <TableBody>
              {rows.map((row, i) => (
                <Row
                  key={i}
                  {...row}
                  isLastRow={i === rows.length - 1}
                  progressBarColor={progressBarColours[i]}
                />
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </CardContent>
    </Card>
  );
};
