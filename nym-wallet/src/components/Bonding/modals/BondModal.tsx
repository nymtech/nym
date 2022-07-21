import React, { useState } from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { NodeTypeSelector } from '../../NodeTypeSelector';
import { GatewayForm } from '../forms/GatewayForm';
import { MixnodeForm } from '../forms/MixnodeForm';

export const BondModal = ({ denom, onClose }: { denom: CurrencyDenom; onClose: () => void }) => {
  const [nodeType, setNodeType] = useState<TNodeType>('mixnode');
  const [step, setStep] = useState<1 | 2>(1);

  const handleBack = step === 2 ? () => setStep(1) : undefined;

  const validateStep = (step: number) => new CustomEvent('validate_step', { detail: { step } });

  const handleSubmit = async () => {};

  return (
    <SimpleModal
      open
      onOk={async () => {
        if (step === 1) window.dispatchEvent(validateStep(1));
        else window.dispatchEvent(validateStep(2));
      }}
      onBack={handleBack}
      onClose={onClose}
      header="Bond"
      subHeader={`Step ${step}/2`}
      okLabel="Next"
    >
      {step === 1 && (
        <Box sx={{ mb: 2 }}>
          <NodeTypeSelector disabled={false} nodeType={nodeType} setNodeType={setNodeType} />
        </Box>
      )}

      <Box sx={{ mb: 2 }}>
        {nodeType === 'mixnode' && (
          <MixnodeForm
            step={step}
            hasVestingTokens={true}
            denom={denom}
            onValidateMixnodeDetail={(data: any) => {
              console.log(data);
              setStep(2);
            }}
            onValidateAmountDetail={(data: any) => {
              console.log(data);
              handleSubmit();
            }}
          />
        )}
        {nodeType === 'gateway' && <GatewayForm step={step} hasVestingTokens={true} />}
      </Box>
    </SimpleModal>
  );
};
