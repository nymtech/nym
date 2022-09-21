<<<<<<< HEAD
import { useState, useEffect } from 'react';
import { Typography, Alert, TextField } from '@mui/material';
import { useTheme } from '@mui/material/styles';
=======
import * as React from 'react';
import { Box, TextField, Typography } from '@mui/material';
import { useEffect, useState } from 'react';
>>>>>>> develop
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
import { ConfirmationModal } from '../../Modals/ConfirmationModal';
import { Error } from '../../Error';

interface Props {
  node: TBondedMixnode | TBondedGateway;
  onConfirm: () => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}

<<<<<<< HEAD
type TUnbondModalStep = 1 | 2 | 3;

const UnbondFirstStep = ({ handleVerification, onClose }: { handleVerification: any; onClose: () => void }) => {
  const [verificationText, setVerificationText] = useState<string>('');
=======
export const UnbondModal = ({ node, onConfirm, onClose, onError }: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();
  const [isConfirmed, setIsConfirmed] = useState(false);
  const [showConfirmModal, setShowConfirmModal] = useState(true);
  const [confirmField, setConfirmField] = useState('');
>>>>>>> develop

  const theme = useTheme();

  return (
    <SimpleModal
      open
      header="Unbond"
      okLabel="Unbond"
      okDisabled={!verificationText.toUpperCase().match(/^UNBOND$/)}
      onOk={handleVerification}
      onClose={onClose}
    >
      <Typography sx={{ fontSize: 14, fontWeight: 600, mb: 3 }}>
        If you unbond your node you will loose all your delegators!
      </Typography>
      <Alert
        severity="error"
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          textAlign: 'center',
        }}
      >
        This action is irreversible and it will not be possible to restore the current state again
      </Alert>
      <Typography sx={{ my: 3, fontSize: 14 }}>
        To unbond, type <span style={{ color: theme.palette.nym.highlight }}>UNBOND</span> in the field below and click
        UNBOND button
      </Typography>
      <TextField
        type="input"
        value={verificationText}
        onChange={(e) => {
          setVerificationText(e.target.value);
        }}
        fullWidth
      />
    </SimpleModal>
  );
};

const UnbondSecondStep = ({ node, onConfirm, onClose, onError }: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();
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

  if (showConfirmModal) {
    return (
      <ConfirmationModal
        title="Unbond"
        confirmButton="UNBOND"
        open={showConfirmModal}
        onConfirm={() => {
          setIsConfirmed(true);
          setShowConfirmModal(false);
        }}
        onClose={onClose}
        disabled={confirmField !== 'UNBOND'}
      >
        <Typography fontWeight={600} mb={2}>
          If you unbond your node you will loose all your delegators!
        </Typography>
        <Error message="This action is irreversible and it will not be possible to restore the current state again" />
        <Typography mt={2} mb={2}>
          To unbond, type{' '}
          <Typography display="inline" component="span" sx={{ color: (t) => t.palette.nym.highlight }}>
            UNBOND
          </Typography>{' '}
          in the field below and click UNBOND button
        </Typography>
        <TextField fullWidth value={confirmField} onChange={(e) => setConfirmField(e.target.value)} />
      </ConfirmationModal>
    );
  }

  if (isConfirmed) {
    return (
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
              node.operatorRewards ? `${node.operatorRewards.amount} ${node.operatorRewards.denom.toUpperCase()}` : '-'
            }
            divider
          />
        )}
        <ModalFee isLoading={isFeeLoading} fee={fee} divider />
        <Typography fontSize="small">Tokens will be transferred to the account you are logged in with now</Typography>
      </SimpleModal>
    );
  }
  return <Box />;
};

export const UnbondModal = ({ node, onConfirm, onClose, onError }: Props) => {
  const [step, setStep] = useState<TUnbondModalStep>(1);

  const handleVerification = () => {
    setStep(2);
  };

  return (
    <div>
      {step === 1 && <UnbondFirstStep handleVerification={handleVerification} onClose={onClose} />}
      {step === 2 && <UnbondSecondStep onClose={onClose} onConfirm={onConfirm} onError={onError} node={node} />}
    </div>
  );
};
