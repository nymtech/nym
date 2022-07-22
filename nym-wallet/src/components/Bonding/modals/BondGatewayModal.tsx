import React, { useState } from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom } from '@nymproject/types';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { useGetFee } from 'src/hooks/useGetFee';
import { MixnodeAmount, MixnodeData } from 'src/pages/bonding/types';
import { simulateBondMixnode } from 'src/requests';
import { TBondMixNodeArgs } from 'src/types';
import { MixnodeForm } from '../forms/MixnodeForm';

const defaultMixnodeValues: MixnodeData = {
  identityKey: '2UB4668XV7qhmJDPp6KLGWGisiaUYThjA4in2o7WKcwA',
  sphinxKey: '5Rh7X4TwMoUwrQ1ivkqWTCGi1pivmHtenaS7VZDUQPYW',
  ownerSignature: '3ccrgwiHhqAbuhhdW7f6UCHZoPFJsQxPcSQRwNc42QVDnDwW8Ebe8p51RhvQp28uqpARysPz52XrE6JuuwJ6fsf8',
  host: '1.1.1.1',
  version: '1.1.1',
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
};

const defaultAmountValues = (denom: CurrencyDenom) => ({
  amount: { amount: '100', denom },
  profitMargin: 10,
  tokenPool: 'balance',
});

export const BondGatewayModal = ({
  denom,
  hasVestingTokens,
  onBondGateway,
  onClose,
}: {
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onBondGateway: () => void;
  onClose: () => void;
}) => {
  const [step, setStep] = useState<1 | 2>(1);
  const [mixnodeData, setMixnodeData] = useState<MixnodeData>(defaultMixnodeValues);
  const [amountData, setAmountData] = useState<MixnodeAmount>(defaultAmountValues(denom));

  const { fee, getFee, resetFeeState } = useGetFee();

  const validateStep = (s: number) => {
    const event = new CustomEvent('validate_mixnode_step', { detail: { step: s } });
    window.dispatchEvent(event);
  };

  const handleOnOK = async () => {
    if (step === 2 && mixnodeData && amountData) {
      getFee<TBondMixNodeArgs>(simulateBondMixnode, {
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
      });
    } else {
      validateStep(step);
    }
  };

  const handleBack = () => {
    validateStep(2);
    setStep(1);
  };

  const handleUpdateMixnodeData = (data: MixnodeData) => {
    setMixnodeData(data);
    setStep(2);
  };

  const handleUpdateAmountData = async (data: MixnodeAmount) => {
    setAmountData(data);
  };

  if (fee) {
    return (
      <ConfirmTx
        open
        header="Delegation details"
        fee={fee}
        onClose={onClose}
        onPrev={resetFeeState}
        onConfirm={async () => {}}
      >
        <ModalListItem label="Node identity key" value={mixnodeData.identityKey} divider />
        <ModalListItem
          label="Amount"
          value={`${amountData.amount.amount} ${amountData.amount.amount.toUpperCase()}`}
          divider
        />
      </ConfirmTx>
    );
  }

  return (
    <SimpleModal
      open
      onOk={handleOnOK}
      onBack={step === 2 ? handleBack : undefined}
      onClose={onClose}
      header="Bond"
      subHeader={`Step ${step}/2`}
      okLabel="Next"
    >
      <Box sx={{ mb: 2 }}>
        <MixnodeForm
          step={step}
          hasVestingTokens={hasVestingTokens}
          denom={denom}
          onValidateMixnodeData={handleUpdateMixnodeData}
          onValidateAmountData={handleUpdateAmountData}
          mixnodeData={mixnodeData}
          amountData={amountData}
        />
      </Box>
    </SimpleModal>
  );
};
