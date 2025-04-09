import { Button, Stack, Typography } from "@mui/material";
import { CurrencyFormField } from "@nymproject/react/currency/CurrencyFormField.js";
import { IdentityKeyFormField } from "@nymproject/react/mixnodes/IdentityKeyFormField.js";
import type { DecCoin } from "@nymproject/types";
import { useCallback, useEffect, useState } from "react";
import SimpleModal from "../../components/modal/SimpleModal";
import useGetWalletBalance from "../../hooks/useGetWalletBalance";
import ExplorerListItem from "../list/ListItem";
import stakingSchema, { MIN_AMOUNT_TO_DELEGATE } from "./schemas";

const StakeModal = ({
  nodeId,
  identityKey,
  onStake,
  onClose,
}: {
  nodeId?: number;
  identityKey?: string;
  onStake: ({
    nodeId,
    amount,
  }: {
    nodeId: number;
    amount: string;
  }) => Promise<void>;
  onClose: () => void;
}) => {
  const { balance } = useGetWalletBalance();
  const [amount, setAmount] = useState<DecCoin>({
    amount: MIN_AMOUNT_TO_DELEGATE,
    denom: "nym",
  });
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();

  const validateAmount = useCallback(async () => {
    try {
      await stakingSchema.parseAsync({
        amount: amount.amount,
        balance,
        nodeId,
      });
      setValidated(true);
      setErrorAmount(undefined);
    } catch (e) {
      if (e instanceof Error && "errors" in e) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const validationError = (e as any).errors; // Explicitly cast if necessary
        console.error(validationError);
        setValidated(false);
        setErrorAmount(validationError[0]?.message);
      } else {
        console.error("Unknown error during validation:", e);
        setValidated(false);
        setErrorAmount("An unexpected error occurred.");
      }
    }
  }, [amount, balance, nodeId]);

  useEffect(() => {
    if (nodeId) {
      validateAmount();
    }
  }, [validateAmount, nodeId]);

  if (!nodeId) {
    return null;
  }

  const handleOnClose = () => {
    setAmount({ amount: MIN_AMOUNT_TO_DELEGATE, denom: "nym" });
    onClose();
  };

  return (
    <SimpleModal
      title="Stake"
      open={!!identityKey}
      onClose={handleOnClose}
      Actions={
        <Button
          variant="contained"
          color="secondary"
          onClick={() => onStake({ nodeId, amount: amount.amount })}
          fullWidth
          disabled={!isValidated}
        >
          Next
        </Button>
      }
    >
      <Stack spacing={3}>
        <Stack spacing={0.5}>
          <Typography variant="body5">Address</Typography>
          <IdentityKeyFormField
            placeholder="Identity Key"
            required
            fullWidth
            initialValue={identityKey}
            readOnly
            showTickOnValid={false}
            sx={{
              "& .MuiInputBase-input": {
                color: "rgba(0, 0, 0, 0.87)",
                "&::placeholder": {
                  color: "rgba(0, 0, 0, 0.54)",
                },
              },
              "& .Mui-disabled": {
                color: "rgba(0, 0, 0, 0.38)",
              },
            }}
          />
        </Stack>
        <Stack spacing={0.5}>
          <Typography variant="body5">Amount</Typography>
          <CurrencyFormField
            placeholder="Amount"
            showCoinMark={false}
            required
            fullWidth
            autoFocus
            initialValue={amount.amount}
            onChanged={setAmount}
            denom={"nym"}
            validationError={errorAmount}
            sx={{
              "& .MuiInputBase-input": {
                color: "rgba(0, 0, 0, 0.87)",
                "&::placeholder": {
                  color: "rgba(0, 0, 0, 0.54)",
                },
              },
              "& .Mui-disabled": {
                color: "rgba(0, 0, 0, 0.38)",
              },
            }}
          />
        </Stack>
        <ExplorerListItem row label="Account balance" value={balance} divider />
      </Stack>
    </SimpleModal>
  );
};

export default StakeModal;
