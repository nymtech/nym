import * as React from "react";
import Box from "@mui/material/Box";
import Collapse from "@mui/material/Collapse";
import IconButton from "@mui/material/IconButton";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import KeyboardArrowDownIcon from "@mui/icons-material/KeyboardArrowDown";
import KeyboardArrowUpIcon from "@mui/icons-material/KeyboardArrowUp";
import { Card, CardContent } from "@mui/material";
import { ExplorerStaticProgressBar } from "./ExplorerStaticProgressBar";

export interface IAccontStatsRowProps {
  type: string;
  allocation: number;
  amount: number;
  value: number;
  history?: { type: string; amount: number }[];
  isLastRow?: boolean;
  progressBarColor: string;
}

const progressBarColours = [
  "#BEF885",
  "#7FB0FF",
  "#00D17D",
  "#004650",
  "#FEECB3",
];

const Row = (props: IAccontStatsRowProps) => {
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
              color={progressBarColor}
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
              {/* Empty cell for alignment */}
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
              align="right"
              sx={{
                borderBottom: "none", // Explicitly remove border
              }}
            >
              {/* Empty cell for alignment */}
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

export const AccountStatsCard = (props: IAccountStatsCardProps) => {
  const { rows } = props;
  return (
    <Card sx={{ height: "100%", borderRadius: "unset" }}>
      <CardContent>
        <TableContainer>
          <Table aria-label="collapsible table" sx={{ marginBottom: 3 }}>
            <TableHead>
              <TableRow>
                <TableCell>Type</TableCell>
                <TableCell align="right">Allocation</TableCell>
                <TableCell align="right">Amount</TableCell>
                <TableCell align="right">Value</TableCell>
                <TableCell></TableCell>
              </TableRow>
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
