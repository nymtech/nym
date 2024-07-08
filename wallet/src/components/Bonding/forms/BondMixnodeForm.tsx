import React from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { NodeTypeSelector } from '@src/components';
import { MixnodeAmount, MixnodeData, Signature } from '@src/pages/bonding/types';
import MixnodeInitForm from './MixnodeInitForm';
import MixnodeAmountForm from './MixnodeAmountForm';
import MixnodeSignatureForm from './MixnodeSignatureForm';

export const BondMixnodeForm = ({
  step,
  denom,
  mixnodeData,
  amountData,
  hasVestingTokens,
  onSelectNodeType,
  onValidateMixnodeData,
  onValidateAmountData,
  onValidateSignature,
}: {
  step: 1 | 2 | 3 | 4;
  mixnodeData: MixnodeData;
  amountData: MixnodeAmount;
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onSelectNodeType: (nodeType: TNodeType) => void;
  onValidateMixnodeData: (data: MixnodeData) => void;
  onValidateAmountData: (data: MixnodeAmount) => Promise<void>;
  onValidateSignature: (signature: Signature) => void;
}) => (
  <>
    {step === 1 && (
      <>
        <Box sx={{ mb: 3 }}>
          <NodeTypeSelector disabled={false} setNodeType={onSelectNodeType} nodeType="mixnode" />
        </Box>
        <MixnodeInitForm onNext={onValidateMixnodeData} mixnodeData={mixnodeData} />
      </>
    )}
    {step === 2 && (
      <MixnodeAmountForm
        denom={denom}
        amountData={amountData}
        hasVestingTokens={hasVestingTokens}
        onNext={onValidateAmountData}
      />
    )}
    {step === 3 && <MixnodeSignatureForm mixnode={mixnodeData} amount={amountData} onNext={onValidateSignature} />}
  </>
);
