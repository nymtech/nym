import { Card, CardContent, Typography, Box, Button } from "@mui/material";
import { ICardDataRowsProps } from "./ExplorerCard";

export interface IStakingCardProps {
  title?: string;
  titleCenter?: string;
  paragraphCenter?: string;
  closeButton?: {
    onClick: () => void;
  };
  addressInput?: boolean;
  amountInput?: boolean;
  dataRows?: ICardDataRowsProps;
  blockExplorerLink?: string;
  backButton?: {
    onClick: () => void;
  };
  nextButton?: {
    onClick: () => void;
  };
}
export const StakingCard = (props: IStakingCardProps) => {
  const {
    title,
    titleCenter,
    closeButton,
    paragraphCenter,
    addressInput,
    amountInput,
    dataRows,
    blockExplorerLink,
    backButton,
    nextButton,
  } = props;
  return (
    <Card sx={{ height: "100%", borderRadius: "unset" }}>
      <CardContent></CardContent>
    </Card>
  );
};
