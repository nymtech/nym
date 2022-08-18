import React, { useEffect, useState } from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { TPoolOption } from 'src/components/TokenPoolSelector';
import { useGetFee } from 'src/hooks/useGetFee';
import { MixnodeAmount, MixnodeData } from 'src/pages/bonding/types';
import { simulateBondMixnode, simulateVestingBondMixnode } from 'src/requests';
import { TBondMixNodeArgs } from 'src/types';
import { BondMixnodeForm } from '../forms/BondMixnodeForm';

const defaultMixnodeValues: MixnodeData = {
  identityKey: '',
  sphinxKey: '',
  ownerSignature: '',
  host: '',
  version: '',
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
};

const defaultAmountValues = (denom: CurrencyDenom) => ({
  amount: { amount: '100', denom },
  profitMargin: 10,
  tokenPool: 'balance',
});

export const BondMixnodeModal = ({
  denom,
  hasVestingTokens,
  onBondMixnode,
  onSelectNodeType,
  onClose,
  onError,
}: {
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onBondMixnode: (data: TBondMixNodeArgs, tokenPool: TPoolOption) => void;
  onSelectNodeType: (type: TNodeType) => void;
  onClose: () => void;
  onError: (e: string) => void;
}) => {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [mixnodeData, setMixnodeData] = useState<MixnodeData>(defaultMixnodeValues);
  const [amountData, setAmountData] = useState<MixnodeAmount>(defaultAmountValues(denom));

  const { fee, getFee, resetFeeState, feeError } = useGetFee();

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const validateStep = async (s: number) => {
    const event = new CustomEvent('validate_bond_mixnode_step', { detail: { step: s } });
    window.dispatchEvent(event);
  };

  const handleBack = () => {
    setStep(1);
  };

  const handleUpdateMixnodeData = (data: MixnodeData) => {
    setMixnodeData(data);
    setStep(2);
  };

  const handleUpdateAmountData = async (data: MixnodeAmount) => {
    setAmountData(data);
    const payload = {
      pledge: data.amount,
      ownerSignature: mixnodeData.ownerSignature,
      mixnode: {
        ...mixnodeData,
        mix_port: mixnodeData.mixPort,
        http_api_port: mixnodeData.httpApiPort,
        verloc_port: mixnodeData.verlocPort,
        sphinx_key: mixnodeData.sphinxKey,
        identity_key: mixnodeData.identityKey,
        profit_margin_percent: data.profitMargin,
      },
    };

    if (data.tokenPool === 'balance') {
      await getFee<TBondMixNodeArgs>(simulateBondMixnode, payload);
    } else {
      await getFee<TBondMixNodeArgs>(simulateVestingBondMixnode, payload);
    }
  };

  const handleConfirm = async () => {
    await onBondMixnode(
      {
        pledge: amountData.amount,
        ownerSignature: mixnodeData.ownerSignature,
        mixnode: {
          ...mixnodeData,
          mix_port: mixnodeData.mixPort,
          http_api_port: mixnodeData.httpApiPort,
          verloc_port: mixnodeData.verlocPort,
          sphinx_key: mixnodeData.sphinxKey,
          identity_key: mixnodeData.identityKey,
          profit_margin_percent: amountData.profitMargin,
        },
      },
      amountData.tokenPool as TPoolOption,
    );
  };

  if (fee) {
    return (
      <ConfirmTx
        open
        header="Bond details"
        fee={fee}
        onClose={onClose}
        onPrev={resetFeeState}
        onConfirm={handleConfirm}
      >
        <ModalListItem label="Node identity key" value={mixnodeData.identityKey} divider />
        <ModalListItem
          label="Amount"
          value={`${amountData.amount.amount} ${amountData.amount.denom.toUpperCase()}`}
          divider
        />
      </ConfirmTx>
    );
  }

  return (
    <SimpleModal
      open
      onOk={async () => {
        await validateStep(step);
      }}
      onBack={step === 2 ? handleBack : undefined}
      onClose={onClose}
      header="Bond mixnode"
      subHeader={`Step ${step}/2`}
      okLabel="Next"
    >
      <Box sx={{ mb: 2 }}>
        <BondMixnodeForm
          step={step}
          denom={denom}
          mixnodeData={mixnodeData}
          amountData={amountData}
          hasVestingTokens={hasVestingTokens}
          onValidateMixnodeData={handleUpdateMixnodeData}
          onValidateAmountData={handleUpdateAmountData}
          onSelectNodeType={onSelectNodeType}
        />
      </Box>
    </SimpleModal>
  );
};
