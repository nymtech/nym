import React, { useCallback, useContext, useState } from 'react';
import { Box, Typography, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, FeeDetails, DecCoin, decimalToFloatApproximation } from '@nymproject/types';
// import { Console } from 'src/utils/console';
// import { useGetFee } from 'src/hooks/useGetFee';
// import { simulateDelegateToMixnode, simulateVestingDelegateToMixnode, tryConvertIdentityToMixId } from 'src/requests';
import { debounce } from 'lodash';
import { SimpleModal } from './SimpleModal';
import { ModalListItem } from './ModalListItem';
// import { AppContext } from 'src/context';
// import { SimpleModal } from '../Modals/SimpleModal';
// import { ModalListItem } from '../Modals/ModalListItem';
// import { checkTokenBalance, validateAmount, validateKey } from '../../utils';
// import { TokenPoolSelector, TPoolOption } from '../TokenPoolSelector';
// import { ConfirmTx } from '../ConfirmTX';

// import { getMixnodeStakeSaturation } from '../../requests';
// import { ErrorModal } from '../Modals/ErrorModal';
// import { BalanceWarning } from '../FeeWarning';

const MIN_AMOUNT_TO_DELEGATE = 10;

export const DelegateModal: FCWithChildren<{
  open: boolean;
  onClose: () => void;
  onOk?: (
    mixId: number,
    identityKey: string,
    amount: DecCoin,
    // tokenPool: TPoolOption,
    // fee?: FeeDetails,
  ) => Promise<void>;
  identityKey?: string;
  onIdentityKeyChanged?: (identityKey: string) => void;
  onAmountChanged?: (amount: string) => void;
  header?: string;
  buttonText?: string;
  rewardInterval: string;
  accountBalance?: string;
  estimatedReward?: number;
  profitMarginPercentage?: string | null;
  nodeUptimePercentage?: number | null;
  denom: CurrencyDenom;
  initialAmount?: string;
  hasVestingContract: boolean;
  sx?: SxProps;
  backdropProps?: object;
}> = ({
  open,
  onIdentityKeyChanged,
  onAmountChanged,
  onClose,
  onOk,
  header,
  buttonText,
  identityKey: initialIdentityKey,
  rewardInterval,
  accountBalance,
  estimatedReward,
  denom,
  profitMarginPercentage,
  nodeUptimePercentage,
  initialAmount,
  hasVestingContract,
  sx,
  backdropProps,
}) => {
  const [mixId, setMixId] = useState<number | undefined>();
  const [identityKey, setIdentityKey] = useState<string | undefined>(initialIdentityKey);
  const [amount, setAmount] = useState<string | undefined>(initialAmount);
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();
  // const [tokenPool, setTokenPool] = useState<TPoolOption>('balance');
  const [errorIdentityKey, setErrorIdentityKey] = useState<string>();
  const [mixIdError, setMixIdError] = useState<string>();

  // const { fee, getFee, resetFeeState, feeError } = useGetFee();
  // const { userBalance } = useContext(AppContext);

  // const handleCheckStakeSaturation = async (newMixId: number) => {
  //   try {
  //     const newSaturation = decimalToFloatApproximation(
  //       (await getMixnodeStakeSaturation(newMixId)).uncapped_saturation,
  //     );
  //     if (newSaturation && newSaturation > 1) {
  //       const saturationPercentage = Math.round(newSaturation * 100);
  //       return { isOverSaturated: true, saturationPercentage };
  //     }
  //     return { isOverSaturated: false, saturationPercentage: undefined };
  //   } catch (e) {
  //     Console.error('Error fetching the saturation, error:', e);
  //     return { isOverSaturated: false, saturationPercentage: undefined };
  //   }
  // };

  const validate = async () => {
    let newValidatedValue = true;
    let errorAmountMessage;
    let errorIdentityKeyMessage;

    // if (!identityKey || !validateKey(identityKey, 32)) {
    //   newValidatedValue = false;
    //   errorIdentityKeyMessage = undefined;
    // }

    // if (identityKey && mixId && validateKey(identityKey, 32)) {
    //   const { isOverSaturated, saturationPercentage } = await handleCheckStakeSaturation(mixId);
    //   if (isOverSaturated) {
    //     newValidatedValue = false;
    //     errorIdentityKeyMessage = `This node is over saturated (${saturationPercentage}%), please select another node`;
    //   }
    // }

    // if (amount && !(await validateAmount(amount, '0'))) {
    //   newValidatedValue = false;
    //   errorAmountMessage = 'Please enter a valid amount';
    // }

    if (amount && Number(amount) < MIN_AMOUNT_TO_DELEGATE) {
      errorAmountMessage = `Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${denom.toUpperCase()}`;
      newValidatedValue = false;
    }

    if (!amount?.length) {
      newValidatedValue = false;
    }

    if (!mixId) {
      newValidatedValue = false;
    }

    setErrorIdentityKey(errorIdentityKeyMessage);
    if (mixIdError && !errorIdentityKeyMessage) {
      setErrorIdentityKey(mixIdError);
    }
    setErrorAmount(errorAmountMessage);
    setValidated(newValidatedValue);
  };

  const handleOk = async () => {
    if (onOk && amount && identityKey && mixId) {
      onOk(mixId, identityKey, { amount, denom }); //tokenPool, fee);
    }
  };

  const handleConfirm = async ({ mixId: id, value }: { mixId: number; value: DecCoin }) => {
    const hasEnoughTokens = true; // await checkTokenBalance(tokenPool, value.amount);

    if (!hasEnoughTokens) {
      setErrorAmount('Not enough funds');
      return;
    }

    //   if (tokenPool === 'balance') {
    //     getFee(simulateDelegateToMixnode, { mixId: id, amount: value });
    //   }

    //   if (tokenPool === 'locked') {
    //     getFee(simulateVestingDelegateToMixnode, { mixId: id, amount: value });
    //   }
  };

  const handleIdentityKeyChanged = (newIdentityKey: string) => {
    setIdentityKey(newIdentityKey);

    if (onIdentityKeyChanged) {
      onIdentityKeyChanged(newIdentityKey);
    }
  };

  const handleAmountChanged = (newAmount: DecCoin) => {
    setAmount(newAmount.amount);

    if (onAmountChanged) {
      onAmountChanged(newAmount.amount);
    }
  };

  React.useEffect(() => {
    validate();
  }, [amount, identityKey, mixIdError]);

  const resolveMixId = useCallback(
    debounce(async (idKey) => {
      if (!idKey) {
        //|| !validateKey(idKey, 32))
        return;
      }
      let res;
      try {
        // res = await tryConvertIdentityToMixId(idKey);
      } catch (e) {
        // Console.warn(`failed to resolve mix_id for "${idKey}": ${e}`);
        return;
      }
      if (res) {
        setMixId(res);
        setMixIdError(undefined);
      } else {
        setMixIdError('Mixnode with this identity does not seem to be currently bonded');
      }
    }, 500),
    [],
  );

  React.useEffect(() => {
    resolveMixId(identityKey);
  }, [identityKey]);

  // if (fee) {
  //   return (
  //     <ConfirmTx
  //       open
  //       header="Delegation details"
  //       fee={fee}
  //       onClose={onClose}
  //       onPrev={resetFeeState}
  //       onConfirm={handleOk}
  //     >
  //       {userBalance.balance?.amount.amount && fee?.amount?.amount && (
  //         <Box sx={{ my: 2 }}>
  //           <BalanceWarning fee={fee?.amount?.amount} tx={amount} />
  //         </Box>
  //       )}
  //       <ModalListItem label="Node identity key" value={identityKey} divider />
  //       <ModalListItem label="Amount" value={`${amount} ${denom.toUpperCase()}`} divider />
  //     </ConfirmTx>
  //   );
  // }

  // if (feeError) {
  //   return (
  //     <ErrorModal
  //       title="Something went wrong while calculating fee. Are you sure you entered a valid node address?"
  //       message={feeError}
  //       sx={sx}
  //       open={open}
  //       onClose={onClose}
  //     />
  //   );
  // }

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={async () => {
        if (mixId && amount) {
          handleConfirm({ mixId, value: { amount, denom } });
        }
      }}
      header={header || 'Delegate'}
      okLabel={buttonText || 'Delegate stake'}
      okDisabled={!isValidated}
      sx={sx}
      backdropProps={backdropProps}
    >
      <Box sx={{ mt: 3 }}>
        <IdentityKeyFormField
          required
          fullWidth
          label="Node identity key"
          onChanged={handleIdentityKeyChanged}
          initialValue={identityKey}
          readOnly={Boolean(initialIdentityKey)}
          textFieldProps={{
            autoFocus: !initialIdentityKey,
          }}
          showTickOnValid={false}
        />
      </Box>
      <Typography
        component="div"
        textAlign="left"
        variant="caption"
        sx={{ color: 'error.main', mx: 2, mt: errorIdentityKey && 1 }}
      >
        {errorIdentityKey}
      </Typography>
      <Box display="flex" gap={2} alignItems="center" sx={{ mt: 3 }}>
        {/* {hasVestingContract && <TokenPoolSelector disabled={false} onSelect={(pool) => setTokenPool(pool)} />} */}
        <CurrencyFormField
          required
          fullWidth
          label="Amount"
          initialValue={amount}
          autoFocus={Boolean(initialIdentityKey)}
          onChanged={handleAmountChanged}
          denom={denom}
          validationError={errorAmount}
        />
      </Box>
      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Account balance" value={accountBalance?.toUpperCase()} divider fontWeight={600} />
      </Box>

      <ModalListItem label="Rewards payout interval" value={rewardInterval} hidden divider />
      <ModalListItem
        label="Node profit margin"
        value={`${profitMarginPercentage ? `${profitMarginPercentage}%` : '-'}`}
        hidden={profitMarginPercentage === undefined}
        divider
      />
      <ModalListItem
        label="Node avg. uptime"
        value={`${nodeUptimePercentage ? `${nodeUptimePercentage}%` : '-'}`}
        hidden={nodeUptimePercentage === undefined}
        divider
      />

      <ModalListItem
        label="Node est. reward per epoch"
        value={`${estimatedReward} ${denom.toUpperCase()}`}
        hidden
        divider
      />
      <ModalListItem label="Est. fee for this transaction will be calculated in the next page" />
    </SimpleModal>
  );
};
