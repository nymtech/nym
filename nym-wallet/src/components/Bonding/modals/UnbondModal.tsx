import { useState } from 'react';
import { Typography, Alert, TextField } from '@mui/material';
import { useEffect } from 'react';
import { TBondedGateway, TBondedMixnode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { isGateway, isMixnode } from 'src/types';
import { ModalFee } from '../../Modals/ModalFee';
import { ModalListItem } from '../../Modals/ModalListItem';
import { SimpleModal } from '../../Modals/SimpleModal';
import {
  simulateUnbondGateway,
  simulateUnbondMixnode,
  simulateVestingUnbondGateway,
  simulateVestingUnbondMixnode,
} from '../../../requests';

interface Props {
  node: TBondedMixnode | TBondedGateway;
  onConfirm: () => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}

type TUnbondModalStep = 1 | 2;

export const UnbondModal = ({ node, onConfirm, onClose, onError }: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();
  const [step, setStep] = useState<TUnbondModalStep>(1);
  const [verificationText, setVerificationText] = useState<string>('');

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  useEffect(() => {
    if (isMixnode(node) && !node.proxy) {
      getFee(simulateUnbondMixnode, {});
    }

    if (isMixnode(node) && node.proxy) {
      getFee(simulateVestingUnbondMixnode, {});
    }

    if (isGateway(node) && !node.proxy) {
      getFee(simulateUnbondGateway, {});
    }

    if (isGateway(node) && node.proxy) {
      getFee(simulateVestingUnbondGateway, {});
    }
  }, [node]);
  console.log('step', step === 1);
  return (
    <div>
      {step === 1 && (
        <SimpleModal
          open
          header="Unbond"
          subHeader="If you unbond your node you will loose all your delegators!"
          okLabel="Unbond"
          okDisabled={Boolean(!verificationText.match('UNBOUND'))}
          onOk={onConfirm}
          onClose={onClose}
        >
          <Alert severity="error">
            This action is irreversible and it will not be possible to restore the current state again
          </Alert>
          <Typography>
            To unbond, type <span style={{ color: 'orange' }}>UNBOND</span> in the field below and click UNBOND button
          </Typography>
          <TextField
            type="input"
            value={verificationText}
            onChange={(e) => setVerificationText(e.target.value)}
            autoFocus
            fullWidth
          />
        </SimpleModal>
      )}
      {step === 2 && (
        <SimpleModal
          open
          header="Unbond"
          subHeader="Unbond and remove your node from the mixnet"
          okLabel="Unbond"
          onOk={onConfirm}
          onClose={onClose}
        >
          <ModalListItem
            label="Amount to unbond"
            value={`${node.bond.amount} ${node.bond.denom.toUpperCase()}`}
            divider
          />
          {isMixnode(node) && (
            <ModalListItem
              label="Operator rewards"
              value={
                node.operatorRewards
                  ? `${node.operatorRewards.amount} ${node.operatorRewards.denom.toUpperCase()}`
                  : '-'
              }
              divider
            />
          )}
          <ModalFee isLoading={isFeeLoading} fee={fee} divider />
          <Typography fontSize="small">Tokens will be transferred to the account you are logged in with now</Typography>
        </SimpleModal>
      )}
    </div>
  );
};
