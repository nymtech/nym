import React, { useEffect, useState } from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { MixnodeForm } from '../forms/MixnodeForm';
import { AmountData, MixnodeData } from 'src/pages/bonding/types';

const defaultMixnodeValues: MixnodeData = {
  identityKey: '2UB4668XV7qhmJDPp6KLGWGisiaUYThjA4in2o7WKcwA',
  sphinxKey: '5Rh7X4TwMoUwrQ1ivkqWTCGi1pivmHtenaS7VZDUQPYW',
  ownerSignature: '3ccrgwiHhqAbuhhdW7f6UCHZoPFJsQxPcSQRwNc42QVDnDwW8Ebe8p51RhvQp28uqpARysPz52XrE6JuuwJ6fsf8',
  host: '1.1.1.1',
  version: '1.1.1',
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
  advancedOpt: false,
};

const defaultAmountValues = (denom: CurrencyDenom): AmountData => ({
  amount: { amount: '100', denom },
  profitMargin: 10,
  tokenPool: 'balance',
});

export const BondMixnodeModal = ({ denom, onClose }: { denom: CurrencyDenom; onClose: () => void }) => {
  const [step, setStep] = useState<1 | 2>(1);
  const [mixnodeData, setMixnodeData] = useState<MixnodeData>(defaultMixnodeValues);
  const [amountData, setAmountData] = useState<AmountData>(defaultAmountValues(denom));

  const handleBack = () => {
    validateStep(2);
    setStep(1);
  };

  const validateStep = (step: number) => {
    const event = new CustomEvent('validate_step', { detail: { step } });
    window.dispatchEvent(event);
  };

  const handleUpdateMixnodeData = (data: MixnodeData) => {
    setMixnodeData(data);
    setStep(2);
  };

  const handleUpdateAmountData = (data: AmountData) => {
    setAmountData(data);
  };

  const handleSubmit = async () => {};

  return (
    <SimpleModal
      open
      onOk={async () => validateStep(step)}
      onBack={step === 2 ? handleBack : undefined}
      onClose={onClose}
      header="Bond"
      subHeader={`Step ${step}/2`}
      okLabel="Next"
    >
      <Box sx={{ mb: 2 }}>
        <MixnodeForm
          step={step}
          hasVestingTokens={true}
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
