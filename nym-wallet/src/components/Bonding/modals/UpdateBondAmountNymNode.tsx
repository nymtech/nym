import React, { useContext, useEffect, useState } from 'react';
import { Box, Stack } from '@mui/material';
import Big from 'big.js';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { DecCoin } from '@nymproject/types';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { useGetFee } from 'src/hooks/useGetFee';
import { decCoinToDisplay, validateAmount } from 'src/utils';
import { simulateUpdateBond, simulateVestingUpdateBond } from 'src/requests';
import { TSimulateUpdateBondArgs, TUpdateBondArgs } from 'src/types';
import { AppContext } from 'src/context';
import { BalanceWarning } from 'src/components/FeeWarning';
import { TBondedNymNode } from 'src/requests/nymNodeDetails';
import { TPoolOption } from '../../TokenPoolSelector';

export const UpdateBondAmountNymNode = ({
  node,
  onUpdateBond,
  onClose,
  onError,
}: {
  node: TBondedNymNode;
  onUpdateBond: (data: TUpdateBondArgs, tokenPool: TPoolOption) => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}) => {
  const { bond: currentBond, stakeSaturation, uncappedStakeSaturation } = node;

  const { fee, getFee, resetFeeState, feeError } = useGetFee();
  const [newBond, setNewBond] = useState<DecCoin | undefined>();
  const [errorAmount, setErrorAmount] = useState(false);

  const { printBalance, printVestedBalance, userBalance } = useContext(AppContext);

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const handleConfirm = async () => {
    if (!newBond) {
      return;
    }

    await onUpdateBond(
      {
        currentPledge: currentBond,
        newPledge: newBond,
        fee: fee?.fee,
      },
      'balance',
    );
  };

  const handleAmountChanged = async (value: DecCoin) => {
    const { amount } = value;
    setNewBond(value);
    if (!amount || !Number(amount)) {
      setErrorAmount(true);
    } else if (Big(amount).eq(currentBond.amount)) {
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
    if (!newBond) {
      return;
    }

    await getFee<TSimulateUpdateBondArgs>(simulateUpdateBond, {
      currentPledge: currentBond,
      newPledge: newBond,
    });
  };

  const newBondToDisplay = () => {
    const coin = decCoinToDisplay(newBond as DecCoin);
    return `${coin.amount} ${coin.denom}`;
  };

  if (fee)
    return (
      <ConfirmTx
        open
        header="Change bond details"
        fee={fee}
        onClose={onClose}
        onPrev={resetFeeState}
        onConfirm={handleConfirm}
      >
        <ModalListItem label="New bond details" value={newBondToDisplay()} divider />
        <ModalListItem label="Change bond details" value={`${currentBond.amount} ${currentBond.denom}`} divider />
        {userBalance.balance?.amount.amount && fee?.amount?.amount && (
          <Box sx={{ my: 2 }}>
            <BalanceWarning fee={fee?.amount?.amount} tx={newBond?.amount} />
          </Box>
        )}
      </ConfirmTx>
    );

  return (
    <SimpleModal
      open
      header="Change bond amount"
      subHeader="Add or reduce amount of tokens on your node"
      okLabel="Next"
      onOk={handleOnOk}
      okDisabled={errorAmount || !newBond}
      onClose={onClose}
    >
      <Stack gap={3}>
        <Box display="flex" gap={1}>
          <CurrencyFormField
            autoFocus
            label="New bond amount"
            denom={currentBond.denom}
            onChanged={(value) => {
              handleAmountChanged(value);
            }}
            fullWidth
            validationError={errorAmount ? 'Please enter a valid amount' : undefined}
          />
        </Box>

        <Box>
          <ModalListItem fontWeight={600} label="Account balance" value={printBalance} divider />
          <ModalListItem label="Current bond amount" value={`${currentBond.amount} ${currentBond.denom}`} divider />
          {uncappedStakeSaturation ? (
            <ModalListItem
              label="Node saturation"
              value={`${uncappedStakeSaturation}%`}
              sxValue={{ color: 'error.main' }}
              divider
            />
          ) : (
            <ModalListItem label="Node saturation" value={`${stakeSaturation}%`} divider />
          )}
          <ModalListItem label="Est. fee for this operation will be calculated in the next page" value="" divider />
        </Box>
      </Stack>
    </SimpleModal>
  );
};
