import SimpleModal from "@/components/modal/SimpleModal";
import useGetWalletBalance from "@/hooks/useGetWalletBalance";
import { validateAmount } from "@/utils/currency";
import { Button, Stack } from "@mui/material";
import { CurrencyFormField } from "@nymproject/react/currency/CurrencyFormField.js";
import { IdentityKeyFormField } from "@nymproject/react/mixnodes/IdentityKeyFormField.js";
import type { DecCoin } from "@nymproject/types";
import { useCallback, useEffect, useState } from "react";
import ExplorerListItem from "../list/ListItem";

const MIN_AMOUNT_TO_DELEGATE = 10;

const StakeModal = ({
  identityKey,
  onClose,
}: {
  identityKey?: string;
  onClose: () => void;
}) => {
  const { balance } = useGetWalletBalance();
  const [amount, setAmount] = useState<DecCoin | undefined>({
    amount: "10",
    denom: "nym",
  });
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();

  const handleDelegate = () => {
    // TODO: Implement
  };

  const validate = useCallback(async () => {
    let newValidatedValue = true;
    let errorAmountMessage = "";

    if (amount && !(await validateAmount(amount.amount, "0"))) {
      newValidatedValue = false;
      errorAmountMessage = "Please enter a valid amount";
    }

    if (amount && +amount.amount < MIN_AMOUNT_TO_DELEGATE) {
      errorAmountMessage = `Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${amount.denom.toUpperCase()}`;
      newValidatedValue = false;
    }

    if (!amount?.amount.length) {
      newValidatedValue = false;
    }

    if (amount && balance && +balance - +amount.amount <= 0) {
      errorAmountMessage = "Not enough funds";
      newValidatedValue = false;
    }

    setErrorAmount(errorAmountMessage);
    setValidated(newValidatedValue);
  }, [amount, balance]);

  const delegateToNymNode = async ({
    delegationMixId,
    delegationAmount,
  }: {
    delegationMixId: number;
    delegationAmount: string;
  }) => {
    // TODO: Implement
  };

  const handleConfirm = async () => {
    // TODO: Implement
  };

  const handleAmountChanged = (newAmount: DecCoin) => {
    setAmount(newAmount);
  };

  return (
    <SimpleModal
      title="Stake"
      open={!!identityKey}
      onClose={onClose}
      Actions={
        <Button
          variant="contained"
          color="secondary"
          onClick={() => undefined}
          fullWidth
        >
          Next
        </Button>
      }
    >
      <Stack spacing={3}>
        <IdentityKeyFormField
          placeholder="Identity Key"
          required
          fullWidth
          onChanged={() => undefined}
          initialValue={identityKey}
          readOnly
          showTickOnValid={false}
        />
        <CurrencyFormField
          placeholder="Amount"
          showCoinMark={false}
          required
          fullWidth
          autoFocus
          initialValue={"10"}
          onChanged={handleAmountChanged}
          denom={"nym"}
          validationError={""}
        />
        <ExplorerListItem row label="Account balance" value={balance} divider />
      </Stack>
    </SimpleModal>
  );
};

export default StakeModal;
