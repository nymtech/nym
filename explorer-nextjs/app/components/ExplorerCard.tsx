import { Card, CardHeader, CardContent, Typography, Box } from "@mui/material";
import React, { FC, ReactElement, ReactEventHandler } from "react";

interface ICardUpDownPriceLineProps {
  percentage: number;
  priceWentUp: boolean;
}
const CardUpDownPriceLine = (
  props: ICardUpDownPriceLineProps
): ReactElement => {
  const { percentage, priceWentUp } = props;
  return (
    <Box>
      <Typography>{percentage} (24H)</Typography>
    </Box>
  );
};

interface ICardTitlePriceProps {
  price: number;
  upDownLine: ICardUpDownPriceLineProps;
}
const CardTitlePrice = (props: ICardTitlePriceProps): React.ReactNode => {
  const { price, upDownLine } = props;
  return (
    <Box>
      <Box>
        <Typography>NYM</Typography>
        <Typography>{price}</Typography>
      </Box>
      <CardUpDownPriceLine {...upDownLine} />
    </Box>
  );
};

interface ICardDataRowsProps {
  rows: Array<{ key: string; value: string }>;
}
const CardDataRows = (props: ICardDataRowsProps): React.ReactNode => {
  const { rows } = props;
  return (
    <Box>
      {rows.map((row, i) => {
        return (
          <Box key={i}>
            <Typography>{row.key}</Typography>
            <Typography>{row.value}</Typography>
          </Box>
        );
      })}
    </Box>
  );
};

type ContentCardProps = {
  overTitle?: string;
  title?: string;
  upDownLine?: ICardUpDownPriceLineProps;
  titlePrice?: ICardTitlePriceProps;
  dataRows?: ICardDataRowsProps;
  graph?: React.ReactNode;
  progressLineGraph?: React.ReactNode;
  paragraph?: string;
  onClick?: ReactEventHandler;
};

export const ExplorerCard: FC<ContentCardProps> = ({
  title,
  titlePrice,
  overTitle,
  upDownLine,
  dataRows,
  graph,
  progressLineGraph,
  onClick,
}) => (
  <Card onClick={onClick} sx={{ height: "100%" }}>
    <CardContent>
      {overTitle && <Typography>{overTitle}</Typography>}
      {title && <Typography>{title}</Typography>}
      {upDownLine && <CardUpDownPriceLine {...upDownLine} />}
      {titlePrice && <CardTitlePrice {...titlePrice} />}
      {dataRows && <CardDataRows {...dataRows} />}
    </CardContent>
  </Card>
);
