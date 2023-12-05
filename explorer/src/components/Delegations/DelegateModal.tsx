import React, { useState, useEffect } from 'react';
import { Box, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, DecCoin } from '@nymproject/types';
import { useChain } from '@cosmos-kit/react';
import { SimpleModal } from './SimpleModal';
import { ModalListItem } from './ModalListItem';
import { DelegationModalProps } from './DelegationModal';
import { unymToNym, validateAmount } from '../../utils/currency';
import { urls } from '../../utils';

const MIN_AMOUNT_TO_DELEGATE = 10;
const MIXNET_CONTRACT_ADDRESS = 'n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr';

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
  const [balance, setBalance] = useState<{
    status: 'loading' | 'success';
    data?: string;
  }>({ status: 'loading', data: undefined });

  const { address, getCosmWasmClient, getSigningCosmWasmClient } = useChain('nyx');

  const getBalance = async (walletAddress: string) => {
    const account = await getCosmWasmClient();
    const uNYMBalance = await account.getBalance(walletAddress, 'unym');
    const NYMBalance = unymToNym(uNYMBalance.amount);

    setBalance({ status: 'success', data: NYMBalance });
  };
  useEffect(() => {
    if (address) {
      getBalance(address);
    }
  }, [address]);

  const validate = async () => {
    let newValidatedValue = true;
    let errorAmountMessage;

    if (amount && !(await validateAmount(amount.amount, '0'))) {
      newValidatedValue = false;
      errorAmountMessage = 'Please enter a valid amount';
    }
    console.log(amount, MIN_AMOUNT_TO_DELEGATE);

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
    delgationAddress,
    delegationAmount,
  }: {
    delegationMixId: number;
    delgationAddress: string;
    delegationAmount: string;
  }) => {
    const amountToDelegate = (Number(delegationAmount) * 1000000).toString();
    const uNymFunds = [{ amount: amountToDelegate, denom: 'unym' }];
    const memo: string = 'test delegation';
    const fee = { gas: '1000000', amount: [{ amount: '1000000', denom: 'unym' }] };

    try {
      const signerClient = await getSigningCosmWasmClient();
      const tx = await signerClient.execute(
        delgationAddress,
        MIXNET_CONTRACT_ADDRESS,
        {
          delegate_to_mixnode: {
            mix_id: delegationMixId,
          },
        },
        fee,
        memo,
        uNymFunds,
      );
      return tx;
    } catch (e) {
      console.error('Failed to delegateToMixnode', e);
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
          delgationAddress: address,
          delegationAmount: amount.amount,
        });

        onOk({
          status: 'success',
          message: 'This operation can take up to one hour to process',
          transactions: [
            { url: `${urls('MAINNET').blockExplorer}/transaction/${tx.transactionHash}`, hash: tx.transactionHash },
          ],
        });
      } catch (e) {
        console.error('Failed to addDelegation', e);
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
