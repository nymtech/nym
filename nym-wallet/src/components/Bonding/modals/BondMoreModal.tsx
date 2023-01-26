import React, { useEffect, useState } from 'react';
import { Box, Stack } from '@mui/material';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { DecCoin } from '@nymproject/types';
import { TPoolOption } from 'src/components/TokenPoolSelector';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { useGetFee } from 'src/hooks/useGetFee';
import { validateAmount } from 'src/utils';
import { simulateBondMore, simulateVestingBondMore } from 'src/requests';
import { TBondMoreArgs } from 'src/types';
import { TBondedMixnode } from 'src/context';

export const BondMoreModal = ({
  node,
  userBalance,
  onBondMore,
  onClose,
  onError,
}: {
  node: TBondedMixnode;
  userBalance?: string;
  onBondMore: (data: TBondMoreArgs, tokenPool: TPoolOption) => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}) => {
  const { bond: currentBond, proxy } = node;
  const { fee, getFee, resetFeeState, feeError } = useGetFee();
  const [additionalBond, setAdditionalBond] = useState<DecCoin>({ amount: '0', denom: currentBond.denom });
  const [errorAmount, setErrorAmount] = useState(false);

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const handleConfirm = async () => {
    const data = { additionalPledge: additionalBond };
    const tokenPool = proxy ? 'locked' : 'balance';
    await onBondMore(data, tokenPool);
  };

  const handleAmountChanged = async (value: DecCoin) => {
    setAdditionalBond(value);
    const { amount } = value;

    if (!amount) {
      setErrorAmount(true);
    } else {
      const validAmount = await validateAmount(amount, '1');
      if (!validAmount) {
        setErrorAmount(true);
        return;
      }
      setErrorAmount(false);
    }
  };

  const handleOnOk = async () => {
    if (!proxy) {
      await getFee<TBondMoreArgs>(simulateBondMore, { additionalPledge: additionalBond });
    } else {
      await getFee<TBondMoreArgs>(simulateVestingBondMore, { additionalPledge: additionalBond });
    }
  };

  if (fee)
    return (
      <ConfirmTx
        open
        header="Bond more details"
        fee={fee}
        onClose={onClose}
        onPrev={resetFeeState}
        onConfirm={handleConfirm}
      >
        <ModalListItem label="Current bond" value={`${currentBond.amount} ${currentBond.denom}`} divider />
        <ModalListItem label="Additional bond" value={`${additionalBond?.amount} ${additionalBond?.denom}`} divider />
      </ConfirmTx>
    );

  return (
    <SimpleModal
      open
      header="Bond more"
      subHeader="Bond more tokens on your node and receive more rewards"
      okLabel="Next"
      onOk={handleOnOk}
      okDisabled={errorAmount}
      onClose={onClose}
    >
      <Stack gap={3}>
        <Box display="flex" gap={1}>
          <CurrencyFormField
            autoFocus
            label="Bond amount"
            denom={currentBond.denom}
            onChanged={(value) => {
              handleAmountChanged(value);
            }}
            fullWidth
            validationError={errorAmount ? 'Please enter a valid amount' : undefined}
          />
        </Box>

        <Box>
          <ModalListItem label="Account balance" value={userBalance?.toUpperCase() || '-'} divider />
          <ModalListItem label="Current bond" value={`${currentBond.amount} ${currentBond.denom}`} divider />
          <ModalListItem label="Est. fee for this operation will be calculated in the next page" value="" divider />
        </Box>
      </Stack>
    </SimpleModal>
  );
};
