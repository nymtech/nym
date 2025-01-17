import SimpleModal from "@/components/modal/SimpleModal";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Button, Stack, Typography } from "@mui/material";

const RedeemRewardsModal = ({
  totalRewardsAmount,
  onRedeem,
  onClose,
}: {
  totalRewardsAmount: number;
  onRedeem: () => Promise<void>;
  onClose: () => void;
}) => {
  const handleOnClose = () => {
    onClose();
  };

  return (
    <SimpleModal
      title="Redeem all rewards"
      open={true}
      onClose={handleOnClose}
      Actions={
        <Button
          variant="contained"
          color="secondary"
          onClick={() => onRedeem()}
          fullWidth
        >
          Next
        </Button>
      }
    >
      <Stack spacing={3}>
        <Stack spacing={0.5}>
          <Typography variant="h3" textAlign={"center"}>
            {`${formatBigNum(totalRewardsAmount / 1_000_000)} NYM`}
          </Typography>
        </Stack>
      </Stack>
    </SimpleModal>
  );
};

export default RedeemRewardsModal;
