"use client";

import CircleIcon from "@mui/icons-material/Circle";
import KeyboardArrowDownIcon from "@mui/icons-material/KeyboardArrowDown";
import KeyboardArrowUpIcon from "@mui/icons-material/KeyboardArrowUp";
import { Card, CardContent } from "@mui/material";
import Box from "@mui/material/Box";
import IconButton from "@mui/material/IconButton";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Typography from "@mui/material/Typography";
import useMediaQuery from "@mui/material/useMediaQuery";
import * as React from "react";
import { MultiSegmentProgressBar } from "../progressBars/MultiSegmentProgressBar";
import { StaticProgressBar } from "../progressBars/StaticProgressBar";

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

const TABLET_WIDTH = "(min-width:700px)";

const Row = (props: IAccontStatsRowProps) => {
  const tablet = useMediaQuery(TABLET_WIDTH);

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
              width: "25%",
            }}
          >
            <Typography>{type}</Typography>
          </TableCell>
          <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
              width: "25%",
            }}
          >
            <Box>
              <Typography>{allocation}%</Typography>
              <StaticProgressBar
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
              width: "20%",
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
              width: "20%",
            }}
          >
            $ {value}
          </TableCell>
          <TableCell
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
              width: "10%",
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
        // MOBILE VIEW
        <TableRow>
          <TableCell
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
              width: "45%",
            }}
          >
            <Box display={"flex"} gap={1} alignItems={"center"}>
              <CircleIcon sx={{ color: progressBarColor }} fontSize="small" />
              {type}
            </Box>
          </TableCell>

          <TableCell
            align="right"
            sx={{
              borderBottom: isLastRow
                ? "none"
                : "1px solid rgba(224, 224, 224, 1)",
              width: "45%",
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
              width: "10%",
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
        history.map((historyRow) => (
          <TableRow key={historyRow.type}>
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
  overTitle?: string;
  priceTitle?: number;
}

export const AccountStatsCard = (props: IAccountStatsCardProps) => {
  const { rows, overTitle, priceTitle } = props;
  const tablet = useMediaQuery(TABLET_WIDTH);
  const progressBarPercentages = () => {
    return rows.map((row) => row.allocation);
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
        {overTitle && (
          <Typography fontSize={14} mb={3} textTransform={"uppercase"}>
            {overTitle}
          </Typography>
        )}
        {priceTitle && (
          <Typography fontSize={24} mb={3}>
            ${priceTitle}
          </Typography>
        )}
        {!tablet && <MultiSegmentProgressBar values={progressValues} />}
        <TableContainer>
          <Table aria-label="collapsible table" sx={{ marginBottom: 3 }}>
            <TableHead>
              {tablet ? (
                <TableRow>
                  <TableCell>
                    <Typography textTransform={"uppercase"}>Type</Typography>
                  </TableCell>
                  <TableCell align="right">
                    <Typography textTransform={"uppercase"}>
                      Allocation
                    </Typography>
                  </TableCell>
                  <TableCell align="right">
                    <Typography textTransform={"uppercase"}>Amount</Typography>
                  </TableCell>
                  <TableCell align="right">
                    <Typography textTransform={"uppercase"}>Value</Typography>
                  </TableCell>
                  <TableCell />
                </TableRow>
              ) : (
                <TableRow>
                  <TableCell>
                    <Typography textTransform={"uppercase"}>Type</Typography>
                  </TableCell>
                  <TableCell align="right">
                    <Typography textTransform={"uppercase"}>
                      Amount / Value
                    </Typography>
                  </TableCell>
                  <TableCell />
                </TableRow>
              )}
            </TableHead>
            <TableBody>
              {rows.map((row, i) => (
                <Row
                  key={row.type}
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
