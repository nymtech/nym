import React, { useState } from 'react';
import { Box, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, DecCoin } from '@nymproject/types';
import { useWalletContext } from '@src/context/wallet';
import { useDelegationsContext } from '@src/context/delegations';
import { SimpleModal } from './SimpleModal';
import { ModalListItem } from './ModalListItem';
import { DelegationModalProps } from './DelegationModal';
import { validateAmount } from '../../utils/currency';
import { urls } from '../../utils';

const MIN_AMOUNT_TO_DELEGATE = 10;

export const DelegateModal: FCWithChildren<{
  mixId: number;
  identityKey: string;
  header?: string;
  buttonText?: string;
  rewardInterval?: string;
  estimatedReward?: number;
  profitMarginPercentage?: string | null;
  nodeUptimePercentage?: number | null;
  denom: CurrencyDenom;
  sx?: SxProps;
  backdropProps?: object;
  onClose: () => void;
  onOk?: (delegationModalProps: DelegationModalProps) => void;
}> = ({ mixId, identityKey, onClose, onOk, denom, sx }) => {
  const [amount, setAmount] = useState<DecCoin | undefined>({ amount: '10', denom: 'nym' });
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();

  const { address, balance } = useWalletContext();
  const { handleDelegate } = useDelegationsContext();

  const validate = async () => {
    let newValidatedValue = true;
    let errorAmountMessage;

    if (amount && !(await validateAmount(amount.amount, '0'))) {
      newValidatedValue = false;
      errorAmountMessage = 'Please enter a valid amount';
    }

    if (amount && +amount.amount < MIN_AMOUNT_TO_DELEGATE) {
      errorAmountMessage = `Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${denom.toUpperCase()}`;
      newValidatedValue = false;
    }

    if (!amount?.amount.length) {
      newValidatedValue = false;
    }

    if (amount && balance.data && +balance.data - +amount.amount <= 0) {
      errorAmountMessage = 'Not enough funds';
      newValidatedValue = false;
    }

    setErrorAmount(errorAmountMessage);
    setValidated(newValidatedValue);
  };

  const delegateToMixnode = async ({
    delegationMixId,
    delegationAmount,
  }: {
    delegationMixId: number;
    delegationAmount: string;
  }) => {
    try {
      const tx = await handleDelegate(delegationMixId, delegationAmount);
      return tx;
    } catch (e) {
      console.error('Failed to delegate to mixnode', e);
      throw e;
    }
  };

  const handleConfirm = async () => {
    if (mixId && amount && onOk) {
      onOk({
        status: 'loading',
      });
      try {
        if (!address) {
          throw new Error('Please connect your wallet');
        }

        const tx = await delegateToMixnode({
          delegationMixId: mixId,
          delegationAmount: amount.amount,
        });

        if (!tx) {
          throw new Error('Failed to delegate');
        }

        onOk({
          status: 'success',
          message: 'This operation can take up to one hour to process',
          transactions: [
            { url: `${urls('MAINNET').blockExplorer}/transaction/${tx.transactionHash}`, hash: tx.transactionHash },
          ],
        });
      } catch (e) {
        console.error('Failed to delegate', e);
        onOk({
          status: 'error',
          message: (e as Error).message,
        });
      }
    }
  };

  const handleAmountChanged = (newAmount: DecCoin) => {
    setAmount(newAmount);
  };

  React.useEffect(() => {
    validate();
  }, [amount, identityKey, mixId]);

  return (
    <SimpleModal
      open
      onClose={onClose}
      onOk={handleConfirm}
      header="Delegate"
      okLabel="Delegate"
      okDisabled={!isValidated}
      sx={sx}
    >
      <Box sx={{ mt: 3 }} gap={2}>
        <IdentityKeyFormField
          required
          fullWidth
          label="Node identity key"
          onChanged={() => undefined}
          initialValue={identityKey}
          readOnly
          showTickOnValid={false}
        />
      </Box>

      <Box display="flex" gap={2} alignItems="center" sx={{ mt: 3 }}>
        <CurrencyFormField
          required
          fullWidth
          autoFocus
          label="Amount"
          initialValue={amount?.amount || '10'}
          onChanged={handleAmountChanged}
          denom={denom}
          validationError={errorAmount}
        />
      </Box>
      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Account balance" value={`${balance.data} NYM`} divider fontWeight={600} />
      </Box>

      <ModalListItem label="Est. fee for this transaction will be calculated in your connected wallet" />
    </SimpleModal>
  );
};
